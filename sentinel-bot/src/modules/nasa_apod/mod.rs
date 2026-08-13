//! Module nasa-apod : publie chaque jour l'« Astronomy Picture of the Day »
//! (APOD) de la NASA dans un salon textuel configure.
//!
//! - Recupere la photo du jour via l'API NASA (cle obligatoire, gratuite sur
//!   api.nasa.gov).
//! - Traduit titre + explication en francais via DeepL si une cle est fournie ;
//!   sinon publie le texte original en anglais (repli).
//! - Poste une fois par jour, a l'heure configuree (defaut 9h UTC). Idempotent :
//!   ne republie pas si la photo du jour est deja dans le salon.

use std::time::Duration;

use chrono::{Timelike, Utc};
use serde::Deserialize;
use serenity::all::{
    Color, CommandInteraction, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse, GetMessages, Permissions,
};
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::prelude::Context;
use tracing::{debug, info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::discord_helpers::{
    get_channel_from_config, guild_config_or_default, is_module_enabled_or_reply_command,
};
use crate::shared::heartbeat::ApiClientKey;

pub const MODULE_BOT_NAME: &str = "nasa-apod-bot";

const MARKER: &str = "NASA · Photo du jour";
const APOD_URL: &str = "https://api.nasa.gov/planetary/apod";
/// Bleu NASA.
const NASA_BLUE: u32 = 0x0B_3D_91;
/// Intervalle de reveil de la boucle (granularite du declenchement horaire).
const TICK: Duration = Duration::from_secs(300);

/// Reponse de l'API APOD (champs utiles seulement).
#[derive(Debug, Deserialize)]
struct Apod {
    date: String,
    title: String,
    #[serde(default)]
    explanation: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    hdurl: Option<String>,
    #[serde(default)]
    media_type: String,
    #[serde(default)]
    copyright: Option<String>,
    #[serde(default)]
    thumbnail_url: Option<String>,
}

/// Lance la boucle de publication quotidienne (une fois par process).
pub fn spawn_background(ctx: Context) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(TICK).await;
            if let Err(e) = tick(&ctx).await {
                debug!(error = %e, "nasa-apod: tick en echec");
            }
        }
    });
}

/// Un passage : pour chaque guild ou le module est actif et dont l'heure de
/// publication correspond a l'heure courante, publie la photo si pas deja fait.
async fn tick(ctx: &Context) -> Result<(), String> {
    let client = {
        let data = ctx.data.read().await;
        data.get::<ApiClientKey>().map(|a| a.client().clone())
    };
    let Some(client) = client else {
        return Ok(()); // API pas encore prete
    };

    let now_hour = Utc::now().hour();
    for gid in ctx.cache.guilds() {
        let guild_id = gid.to_string();
        let cfg = guild_config_or_default(ctx, &guild_id, MODULE_BOT_NAME).await;
        if !BaseApiClient::config_bool(&cfg, "enabled", false) {
            continue;
        }
        // Heure de publication exprimee dans le fuseau local du serveur :
        // `post_hour` locale, `timezone_offset` = decalage vs UTC (ex. +1 Paris
        // hiver, +2 ete). On ramene a l'heure UTC equivalente pour comparer.
        let post_hour = BaseApiClient::config_u64(&cfg, "post_hour", 9);
        let offset = cfg
            .get("timezone_offset")
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(0);
        let target_utc =
            platform_core::sentinel::domain::services::system::scheduling::local_hour_to_utc(
                post_hour, offset,
            );
        if now_hour != target_utc {
            continue;
        }
        let api_key = BaseApiClient::config_or(&cfg, "nasa_api_key", "");
        if api_key.trim().is_empty() {
            continue; // cle obligatoire : rien sans elle
        }
        let Some(channel) =
            get_channel_from_config(ctx, &guild_id, MODULE_BOT_NAME, "channel_id").await
        else {
            continue;
        };

        // Recupere la photo du jour.
        let apod = match fetch_apod(&client, api_key.trim()).await {
            Ok(a) => a,
            Err(e) => {
                warn!(guild_id = %guild_id, error = %e, "nasa-apod: recuperation APOD echouee");
                continue;
            }
        };

        // Idempotence : la photo du jour est-elle deja postee dans ce salon ?
        let marker = format!("{MARKER} · {}", apod.date);
        if already_posted(ctx, channel, &marker).await {
            continue;
        }

        // Traduction FR optionnelle (repli : texte anglais).
        let deepl_key = BaseApiClient::config_or(&cfg, "deepl_api_key", "");
        let (title, explanation) =
            translate_or_original(&client, deepl_key.trim(), &apod.title, &apod.explanation).await;

        let embed = build_embed(&apod, &title, &explanation, &marker);
        if let Err(e) = channel
            .send_message(&ctx.http, CreateMessage::new().embed(embed))
            .await
        {
            warn!(guild_id = %guild_id, error = %e, "nasa-apod: publication echouee");
            continue;
        }
        info!(guild_id = %guild_id, date = %apod.date, "nasa-apod: photo du jour publiee");
    }
    Ok(())
}

/// Commande slash du module.
pub fn register_commands() -> Vec<CreateCommand> {
    vec![CreateCommand::new("apod")
        .description("Affiche la photo de l'espace du jour (NASA)")
        .default_member_permissions(Permissions::empty())]
}

/// `/apod` : publie a la demande la photo du jour dans le salon courant.
/// Accessible a tous ; utile pour tester ou revoir la photo sans attendre
/// l'heure de publication automatique.
pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
    let Some(gid) = command.guild_id else {
        return;
    };
    let cfg = guild_config_or_default(ctx, &gid.to_string(), MODULE_BOT_NAME).await;
    let api_key = BaseApiClient::config_or(&cfg, "nasa_api_key", "");
    if api_key.trim().is_empty() {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().ephemeral(true).content(
                        "⚠️ Aucune clé API NASA configurée. Un admin doit la renseigner dans les Composants.",
                    ),
                ),
            )
            .await;
        return;
    }

    // La recuperation + traduction peut prendre 1-2 s : on differe la reponse.
    if command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(Default::default()),
        )
        .await
        .is_err()
    {
        return;
    }

    let client = {
        let data = ctx.data.read().await;
        data.get::<ApiClientKey>().map(|a| a.client().clone())
    };
    let Some(client) = client else {
        let _ = command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("API indisponible, réessaie plus tard."),
            )
            .await;
        return;
    };

    let apod = match fetch_apod(&client, api_key.trim()).await {
        Ok(a) => a,
        Err(e) => {
            warn!(error = %e, "nasa-apod: /apod recuperation echouee");
            let _ = command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new()
                        .content("Impossible de récupérer la photo du jour de la NASA."),
                )
                .await;
            return;
        }
    };

    let deepl_key = BaseApiClient::config_or(&cfg, "deepl_api_key", "");
    let (title, explanation) =
        translate_or_original(&client, deepl_key.trim(), &apod.title, &apod.explanation).await;
    let marker = format!("{MARKER} · {}", apod.date);
    let embed = build_embed(&apod, &title, &explanation, &marker);
    let _ = command
        .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
        .await;
}

/// Appelle l'API APOD de la NASA.
async fn fetch_apod(client: &reqwest::Client, api_key: &str) -> Result<Apod, String> {
    let resp = client
        .get(APOD_URL)
        .query(&[("api_key", api_key), ("thumbs", "true")])
        .send()
        .await
        .map_err(|e| format!("requete: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("statut HTTP {}", resp.status()));
    }
    resp.json::<Apod>().await.map_err(|e| format!("json: {e}"))
}

/// Vrai si un message recent du salon porte deja le marqueur du jour.
async fn already_posted(ctx: &Context, channel: serenity::all::ChannelId, marker: &str) -> bool {
    match channel
        .messages(&ctx.http, GetMessages::new().limit(20))
        .await
    {
        Ok(msgs) => msgs.iter().any(|m| {
            m.embeds.iter().any(|e| {
                e.footer
                    .as_ref()
                    .map(|f| f.text.contains(marker))
                    .unwrap_or(false)
            })
        }),
        Err(_) => false, // en cas de doute on ne bloque pas (au pire un doublon)
    }
}

/// Traduit (titre, explication) en francais via DeepL si une cle est fournie.
/// En l'absence de cle ou en cas d'echec : renvoie les textes d'origine.
async fn translate_or_original(
    client: &reqwest::Client,
    deepl_key: &str,
    title: &str,
    explanation: &str,
) -> (String, String) {
    if deepl_key.is_empty() {
        return (title.to_string(), explanation.to_string());
    }
    match translate_batch(client, deepl_key, &[title, explanation]).await {
        Some(mut out) if out.len() == 2 => {
            let expl = out.pop().unwrap();
            let titl = out.pop().unwrap();
            (titl, expl)
        }
        _ => (title.to_string(), explanation.to_string()),
    }
}

#[derive(Deserialize)]
struct DeeplResp {
    translations: Vec<DeeplItem>,
}
#[derive(Deserialize)]
struct DeeplItem {
    text: String,
}

/// Traduit un lot de textes vers le francais. `None` si l'appel echoue.
async fn translate_batch(
    client: &reqwest::Client,
    deepl_key: &str,
    texts: &[&str],
) -> Option<Vec<String>> {
    // Les cles gratuites se terminent par ":fx" et utilisent api-free.
    let endpoint = if deepl_key.ends_with(":fx") {
        "https://api-free.deepl.com/v2/translate"
    } else {
        "https://api.deepl.com/v2/translate"
    };
    let mut form: Vec<(&str, &str)> = vec![("target_lang", "FR"), ("source_lang", "EN")];
    for t in texts {
        form.push(("text", t));
    }
    let resp = client
        .post(endpoint)
        .header("Authorization", format!("DeepL-Auth-Key {deepl_key}"))
        .form(&form)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        debug!(status = %resp.status(), "nasa-apod: DeepL a refuse la traduction");
        return None;
    }
    let parsed: DeeplResp = resp.json().await.ok()?;
    Some(parsed.translations.into_iter().map(|t| t.text).collect())
}

/// Construit l'embed a publier a partir de la photo et des textes (traduits).
fn build_embed(apod: &Apod, title: &str, explanation: &str, marker: &str) -> CreateEmbed {
    // Discord limite la description a 4096 caracteres.
    let mut desc: String = explanation.chars().take(4000).collect();
    if explanation.chars().count() > 4000 {
        desc.push('…');
    }

    let footer = match &apod.copyright {
        Some(c) if !c.trim().is_empty() => format!("{marker} · © {}", c.trim()),
        _ => marker.to_string(),
    };

    let mut embed = CreateEmbed::new()
        .title(title)
        .description(desc)
        .color(Color::new(NASA_BLUE))
        .footer(serenity::builder::CreateEmbedFooter::new(footer));

    // Image : pour une image, la grande version ; pour une video, la vignette.
    if apod.media_type == "image" {
        let img = apod.hdurl.clone().unwrap_or_default();
        let img = if img.is_empty() {
            apod.url.clone()
        } else {
            img
        };
        if !img.is_empty() {
            embed = embed.image(img);
        }
    } else {
        if let Some(thumb) = &apod.thumbnail_url {
            if !thumb.is_empty() {
                embed = embed.image(thumb.clone());
            }
        }
        if !apod.url.is_empty() {
            embed = embed.url(&apod.url);
        }
    }
    embed
}
