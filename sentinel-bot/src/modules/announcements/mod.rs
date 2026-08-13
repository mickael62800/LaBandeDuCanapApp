//! Consumer stream : poste sur Discord les annonces planifiees publiees
//! par announcement-worker.
//!
//! Flow :
//! 1. announcement-worker tick chaque heure pile, fetch les annonces dues
//!    depuis l'API, XADD `sentinel:events` event="announcement_publish".
//! 2. Le bot (ce module) consume via event_bus::listen_stream_group,
//!    poste sur chaque channel cible (text simple ou embed riche),
//!    rapporte le resultat (channel_id, message_id, success) a l'API
//!    via `AnnouncementsService::RecordRunResult` (gRPC). Les clics de
//!    boutons remontent via `RecordButtonClick`.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serenity::all::{ButtonStyle, Color, ComponentInteraction, ReactionType};
use serenity::builder::{
    CreateActionRow, CreateAllowedMentions, CreateButton, CreateEmbed, CreateEmbedAuthor,
    CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::id::{ChannelId, RoleId};
use serenity::prelude::*;

/// Extrait les RoleId des mentions `<@&123>` d'un texte (le prefix d'annonce,
/// construit cote serveur), pour n'autoriser que ces roles dans AllowedMentions.
fn extract_role_ids(s: &str) -> Vec<RoleId> {
    let mut ids = Vec::new();
    let mut rest = s;
    while let Some(pos) = rest.find("<@&") {
        let after = &rest[pos + 3..];
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(id) = digits.parse::<u64>() {
            ids.push(RoleId::new(id));
        }
        rest = after;
    }
    ids
}
use tracing::{info, warn};

use crate::shared::grpc_client::{grpc_err_to_string, GrpcClientKey, SentinelGrpcClient};
use platform_proto::sentinel::announcements::v1 as proto_ann;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RenderedEmbed {
    title: Option<String>,
    description: String,
    color: Option<i32>,
    image_url: Option<String>,
    thumbnail_url: Option<String>,
    /// Texte de pied : Discord le rend SOUS l'image de l'embed.
    #[serde(default)]
    footer_text: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AnnouncementButton {
    label: String,
    style: String,
    custom_id: Option<String>,
    url: Option<String>,
    emoji: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RenderedAnnouncement {
    announcement_id: String,
    run_id: String,
    guild_id: String,
    channel_ids: Vec<String>,
    content_text: String,
    embed: Option<RenderedEmbed>,
    mentions_prefix: String,
    #[serde(default)]
    buttons: Vec<AnnouncementButton>,
    #[serde(default)]
    auto_reactions: Vec<String>,
}

#[derive(Debug)]
struct ChannelPostResult {
    channel_id: String,
    message_id: Option<String>,
    success: bool,
    error: Option<String>,
}

/// Spawn le consumer durable. Appele une fois au `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "sentinel-bot-announcements".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move { handle_event(&ctx, &payload_json).await }
            },
        )
        .await;
    });
}

async fn handle_event(ctx: &Context, payload_json: &str) {
    let envelope: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    if envelope.get("event").and_then(|v| v.as_str()) != Some("announcement_publish") {
        return;
    }
    let data = match envelope.get("data") {
        Some(d) => d.clone(),
        None => return,
    };
    let payload: RenderedAnnouncement = match serde_json::from_value(data) {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "announcement_publish: data invalide");
            return;
        }
    };

    info!(
        run_id = %payload.run_id,
        announcement_id = %payload.announcement_id,
        channels = payload.channel_ids.len(),
        "Posting announcement"
    );

    let mut results: Vec<ChannelPostResult> = Vec::with_capacity(payload.channel_ids.len());

    for ch_id_str in &payload.channel_ids {
        let result = post_to_channel(ctx, ch_id_str, &payload).await;
        results.push(result);
    }

    // Rapporte le resultat a l'API
    let grpc = {
        let data = ctx.data.read().await;
        data.get::<GrpcClientKey>().cloned()
    };
    if let Some(grpc) = grpc {
        report_run_result(&grpc, &payload.run_id, &results).await;
    } else {
        warn!("GrpcClientKey absent, impossible de reporter le resultat du run");
    }
}

/// Prefix unique pour les custom_id des boutons d'annonces. Permet de
/// reconnaitre les interactions destinees a ce module et d'extraire les
/// IDs (announcement_id, run_id, user_custom_id) pour les rapporter.
pub const BUTTON_CUSTOM_ID_PREFIX: &str = "ann:";

fn build_button_custom_id(announcement_id: &str, run_id: &str, user_id: &str) -> String {
    format!(
        "{}{}:{}:{}",
        BUTTON_CUSTOM_ID_PREFIX, announcement_id, run_id, user_id
    )
}

/// Decompose un custom_id genere par build_button_custom_id.
/// Retourne (announcement_id, run_id, user_button_id) si valide.
pub fn parse_button_custom_id(cid: &str) -> Option<(String, String, String)> {
    let rest = cid.strip_prefix(BUTTON_CUSTOM_ID_PREFIX)?;
    let mut parts = rest.splitn(3, ':');
    let ann = parts.next()?;
    let run = parts.next()?;
    let btn = parts.next()?;
    Some((ann.to_string(), run.to_string(), btn.to_string()))
}

pub fn handles_component(cid: &str) -> bool {
    cid.starts_with(BUTTON_CUSTOM_ID_PREFIX)
}

fn parse_button_style(s: &str) -> ButtonStyle {
    match s {
        "primary" => ButtonStyle::Primary,
        "success" => ButtonStyle::Success,
        "danger" => ButtonStyle::Danger,
        "link" => ButtonStyle::Primary, // les liens sont traites a part avec ::link()
        _ => ButtonStyle::Secondary,
    }
}

fn build_action_rows(
    buttons: &[AnnouncementButton],
    announcement_id: &str,
    run_id: &str,
) -> Vec<CreateActionRow> {
    if buttons.is_empty() {
        return Vec::new();
    }
    let row_buttons: Vec<CreateButton> = buttons
        .iter()
        .take(5) // Discord max 5 par row
        .filter_map(|b| {
            // Cas link
            if b.style == "link" {
                let url = b.url.as_ref()?;
                if url.is_empty() {
                    return None;
                }
                let mut btn = CreateButton::new_link(url.clone()).label(b.label.clone());
                if let Some(emoji) = &b.emoji {
                    if !emoji.is_empty() {
                        if let Some(rt) = parse_emoji(emoji) {
                            btn = btn.emoji(rt);
                        }
                    }
                }
                return Some(btn);
            }
            // Cas action : custom_id requis
            let user_btn_id = b.custom_id.as_ref().filter(|s| !s.is_empty())?;
            let cid = build_button_custom_id(announcement_id, run_id, user_btn_id);
            let mut btn = CreateButton::new(cid)
                .label(b.label.clone())
                .style(parse_button_style(&b.style));
            if let Some(emoji) = &b.emoji {
                if !emoji.is_empty() {
                    if let Some(rt) = parse_emoji(emoji) {
                        btn = btn.emoji(rt);
                    }
                }
            }
            Some(btn)
        })
        .collect();
    if row_buttons.is_empty() {
        Vec::new()
    } else {
        vec![CreateActionRow::Buttons(row_buttons)]
    }
}

/// Parse un emoji unicode ou custom Discord <:name:id> ou <a:name:id>.
/// Délègue au parseur unique (`modules::emoji`, adossé au core) — l'ancienne
/// copie locale cassait sur un nom contenant `:`.
fn parse_emoji(s: &str) -> Option<ReactionType> {
    crate::modules::emoji::parse_reaction_type(s)
}

async fn post_to_channel(
    ctx: &Context,
    ch_id_str: &str,
    payload: &RenderedAnnouncement,
) -> ChannelPostResult {
    let ch_id = match ch_id_str.parse::<u64>() {
        Ok(id) => ChannelId::new(id),
        Err(e) => {
            return ChannelPostResult {
                channel_id: ch_id_str.to_string(),
                message_id: None,
                success: false,
                error: Some(format!("channel_id invalide: {e}")),
            };
        }
    };

    // Construit le message : mentions_prefix + content_text (si pas d'embed)
    // ou mentions_prefix seul (si embed, le contenu va dans la description).
    let mut msg = CreateMessage::new();

    let prefix = payload.mentions_prefix.trim();
    let body = if let Some(ref embed) = payload.embed {
        let mut e = CreateEmbed::new().description(&embed.description);
        if let Some(t) = &embed.title {
            e = e.title(t.clone());
            // Petit fallback pour avoir un author/header sympa
            e = e.author(CreateEmbedAuthor::new(t));
        }
        if let Some(c) = embed.color {
            e = e.color(Color::new(c as u32));
        }
        if let Some(url) = &embed.thumbnail_url {
            if !url.is_empty() {
                e = e.thumbnail(url.clone());
            }
        }
        // L'image est integree a l'embed : un seul message est envoye, Discord
        // affiche l'image en grand sous le texte.
        if let Some(url) = &embed.image_url {
            if !url.is_empty() {
                e = e.image(url.clone());
            }
        }
        // Le footer se place SOUS l'image : c'est le "texte du bas".
        if let Some(footer) = &embed.footer_text {
            if !footer.is_empty() {
                e = e.footer(CreateEmbedFooter::new(footer.clone()));
            }
        }
        msg = msg.embed(e);
        if !prefix.is_empty() {
            msg = msg.content(prefix.to_string());
        }
        Ok::<(), String>(())
    } else {
        let combined = if prefix.is_empty() {
            payload.content_text.clone()
        } else {
            format!("{}\n{}", prefix, payload.content_text)
        };
        msg = msg.content(combined);
        Ok(())
    };

    // body est juste un Result vide pour absorber les erreurs eventuelles
    let _ = body;

    // AllowedMentions derive du PREFIX (construit cote serveur a partir des flags
    // de l'annonce) : on n'autorise @everyone/@here et un role QUE s'ils sont
    // reellement presents dans le prefix voulu. Sans cela (aucun AllowedMentions),
    // Discord parse et declenche TOUTES les mentions du content_text arbitraire
    // -> un mod sans permission "mention everyone" pouvait pinguer tout le serveur.
    let allow_everyone = prefix.contains("@everyone") || prefix.contains("@here");
    let allowed = CreateAllowedMentions::new()
        .everyone(allow_everyone)
        .roles(extract_role_ids(prefix));
    msg = msg.allowed_mentions(allowed);

    // Ajout des boutons (max 1 row Discord = 5 boutons)
    let rows = build_action_rows(&payload.buttons, &payload.announcement_id, &payload.run_id);
    if !rows.is_empty() {
        msg = msg.components(rows);
    }

    match ch_id.send_message(&ctx.http, msg).await {
        Ok(message) => {
            // Ajoute les reactions automatiques (best-effort, ne bloque pas)
            for emoji_str in &payload.auto_reactions {
                if let Some(rt) = parse_emoji(emoji_str) {
                    if let Err(e) = message.react(&ctx.http, rt).await {
                        warn!(error = %e, emoji = %emoji_str, "Echec ajout reaction");
                    }
                }
            }
            ChannelPostResult {
                channel_id: ch_id_str.to_string(),
                message_id: Some(message.id.to_string()),
                success: true,
                error: None,
            }
        }
        Err(e) => {
            warn!(error = %e, channel_id = ch_id_str, "Echec envoi annonce");
            ChannelPostResult {
                channel_id: ch_id_str.to_string(),
                message_id: None,
                success: false,
                error: Some(e.to_string()),
            }
        }
    }
}

// ── Handler component interaction ───────────────────────────────────────

/// Appele depuis interaction_create quand custom_id commence par "ann:".
/// Reponse ephemere "Cliqué !" + report a l'API pour tracking.
pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.as_str();
    let (announcement_id, run_id, button_user_id) = match parse_button_custom_id(cid) {
        Some(t) => t,
        None => return,
    };

    // Reponse ephemere immediate (Discord exige une reponse < 3s)
    let label = button_user_id.clone();
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(format!("✅ Tu as cliqué sur **{}**", label))
            .ephemeral(true),
    );
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec reponse ephemere bouton annonce");
    }

    // Report a l'API
    let grpc = {
        let data = ctx.data.read().await;
        data.get::<GrpcClientKey>().cloned()
    };
    if let Some(grpc) = grpc {
        let req = proto_ann::RecordButtonClickRequest {
            announcement_id,
            run_id: Some(run_id),
            user_id: component.user.id.to_string(),
            user_name: Some(component.user.name.clone()),
            button_custom_id: button_user_id,
            button_label: Some(label),
        };
        if let Err(e) = crate::grpc_call!(@unit &grpc, announcements, record_button_click, req) {
            warn!(error = %e, "Echec report button-click a l'API");
        }
    }
}

async fn report_run_result(
    grpc: &Arc<SentinelGrpcClient>,
    run_id: &str,
    results: &[ChannelPostResult],
) {
    let req = proto_ann::RecordRunResultRequest {
        run_id: run_id.to_string(),
        channels_posted: results
            .iter()
            .map(|r| proto_ann::ChannelPostResult {
                channel_id: r.channel_id.clone(),
                message_id: r.message_id.clone(),
                success: r.success,
                error: r.error.clone(),
            })
            .collect(),
    };
    match crate::grpc_call!(@unit grpc, announcements, record_run_result, req) {
        Ok(_) => info!(run_id, "Run result reported"),
        Err(e) => warn!(run_id, error = %e, "Echec report run result"),
    }
}
