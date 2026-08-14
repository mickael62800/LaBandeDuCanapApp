//! Game Portal : projection Discord d'un serveur de jeu.
//!
//! Le module est piloté par les événements publiés par `nexus-api` sur la
//! stream Redis `nexus:events` :
//!   - `game_server_scheduled` : ouverture programmee — cree les memes salons et
//!     le panneau que `game_server_started`, mais le conteneur reste eteint (le
//!     worker le demarre ~5 min avant l'ouverture) ;
//!   - `game_server_started` : cree un salon texte + un salon vocal PRIVES
//!     (visibles du seul role du jeu) dans la categorie configuree, epingle un
//!     panneau avec bouton d'inscription et ping le role ;
//!   - `game_server_stopped` : arret temporaire — les salons, le role et le
//!     panneau sont CONSERVES (pour pouvoir redemarrer sans tout reconstruire) ;
//!   - `game_server_deleted` : suppression du jeu — supprime salons et role ;
//!   - `game_ip_reveal` : poste l'adresse de connexion et rafraichit le panneau ;
//!   - `game_daily_ping` : rappelle l'ouverture a venir au role du jeu.
//!
//! Les joueurs consultent les reglages d'un serveur via la commande ephemere
//! `/game parametres` (voir `params_embeds_for_channel`), pas via une carte
//! epinglee.
//!
//! La configuration par guild (categorie, hote public) est lue via
//! `GET /api/config/{guild_id}/game-portal`.

use std::sync::Arc;

use serenity::all::{
    ButtonStyle, ChannelId, ChannelType, Colour, ComponentInteraction, Context, CreateActionRow,
    CreateButton, CreateChannel, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditMessage, EditRole, GuildId,
    PermissionOverwrite, PermissionOverwriteType, Permissions, RoleId,
};

use crate::api_client::{ApiClient, GameServer};

/// Nom du module dans `bot_guild_config` (cle de lecture de la config guild).
const MODULE_BOT_NAME: &str = "game-portal";

/// custom_id du bouton d'inscription : `gp_register:{server_id}`.
pub const REGISTER_PREFIX: &str = "gp_register:";
pub const REVEAL_IP_PREFIX: &str = "gp_reveal_ip:";

pub fn handles_component(custom_id: &str) -> bool {
    custom_id.starts_with(REGISTER_PREFIX) || custom_id.starts_with(REVEAL_IP_PREFIX)
}

pub async fn on_component(api: &ApiClient, ctx: &Context, component: &ComponentInteraction) {
    if let Some(server_id) = component.data.custom_id.strip_prefix(REVEAL_IP_PREFIX) {
        on_reveal_ip_component(api, ctx, component, server_id).await;
        return;
    }

    let Some(server_id) = component.data.custom_id.strip_prefix(REGISTER_PREFIX) else {
        return;
    };

    let reg_result = api
        .register_to_server(server_id, &component.user.id.to_string())
        .await;
    // L'API peut refuser (serveur ferme, capacite, etc.) : on ne pretend pas
    // que l'inscription a reussi -> message ephemere et on s'arrete.
    if let Err(e) = reg_result {
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!("❌ Inscription impossible : {e}"))
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    // Re-fetch inscrits + serveur pour reconstruire le panneau.
    let user_ids: Vec<String> = api
        .list_server_registrations(server_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.user_id)
        .collect();

    if let Ok(detail) = api.get_game_server(server_id).await {
        let template = api.get_game_template(&detail.server.template_id).await.ok();
        let game_name = template
            .as_ref()
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Jeu".into());
        let cover_url = public_cover_url(
            template
                .as_ref()
                .and_then(|template| template.cover_image_url.as_deref()),
        );
        grant_session_role(ctx, component, server_id, &game_name).await;
        let embed = build_public_panel_embed(
            &game_name,
            &detail.server.name,
            &user_ids,
            detail.server.ip_reveal_at.as_deref(),
            detail.server.ip_revealed,
            cover_url.as_deref(),
        );
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(vec![panel_row(server_id, detail.server.ip_revealed)]),
                ),
            )
            .await;
        return;
    }

    // Fallback : simple accuse ephemere.
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("✅ Inscription enregistree.")
                    .ephemeral(true),
            ),
        )
        .await;
}

fn session_suffix(server_id: &str) -> String {
    server_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect()
}

fn session_role_name(game_name: &str, server_id: &str) -> String {
    format!("{}_{}", slugify(game_name), session_suffix(server_id))
}

fn registration_channel_name(game_name: &str) -> String {
    format!("inscription-{}", slugify(game_name))
}

fn private_text_name(game_name: &str) -> String {
    format!("salon-{}", slugify(game_name))
}

fn legacy_private_text_name(server_id: &str) -> String {
    format!("joueurs-{}", session_suffix(server_id))
}

fn private_text_topic(server_id: &str) -> String {
    format!("Nexus Game Portal | session:{server_id} | private")
}

fn is_player_password_key(key: &str) -> bool {
    matches!(
        key.to_ascii_uppercase().as_str(),
        "PASSWORD" | "SERVER_PASS" | "SERVER_PASSWORD" | "SERVERCONFIG_SERVERPASSWORD"
    )
}

fn is_safe_game_option(key: &str) -> bool {
    let key = key.to_ascii_uppercase();
    if is_player_password_key(&key) {
        return true;
    }
    ![
        "ADMIN",
        "RCON",
        "TOKEN",
        "SECRET",
        "PRIVATE_KEY",
        "API_KEY",
        "ACCESS_KEY",
        "OPERATOR",
        "OP_PERMISSION",
    ]
    .iter()
    .any(|forbidden| key.contains(forbidden))
        && key != "OPS"
}

fn public_game_options(config: &std::collections::HashMap<String, String>) -> Vec<String> {
    let mut options: Vec<_> = config
        .iter()
        .filter(|(key, _)| is_safe_game_option(key))
        .map(|(key, value)| {
            let shown = if is_player_password_key(key) && value.trim().is_empty() {
                "Aucun (accès libre)"
            } else {
                value.as_str()
            };
            format!("**{key}** : `{shown}`")
        })
        .collect();
    options.sort_unstable();
    options
}

/// Footer des embeds de la commande `/game parametres`.
const OPTIONS_FOOTER: &str = "Game Portal | Paramètres";

/// Options affichées hors salon privé (ex. salon d'inscription) : comme
/// `public_game_options` mais SANS le mot de passe. Ce salon est visible de tout
/// le rôle du jeu ; le mot de passe ne doit apparaître qu'au salon privé des
/// inscrits.
fn registration_options(config: &std::collections::HashMap<String, String>) -> Vec<String> {
    let mut options: Vec<_> = config
        .iter()
        .filter(|(key, _)| is_safe_game_option(key) && !is_player_password_key(key))
        .map(|(key, value)| format!("**{key}** : `{value}`"))
        .collect();
    options.sort_unstable();
    options
}

/// Construit la carte des paramètres (un ou plusieurs embeds, un par chunk).
/// Bornée à 10 embeds (limite Discord par message).
fn build_options_embeds(
    game_name: &str,
    server_name: &str,
    options: &[String],
) -> Vec<CreateEmbed> {
    let mut chunks = chunk_options(options);
    if chunks.is_empty() {
        chunks.push("_Aucune option publique configurée._".into());
    }
    chunks.truncate(10);
    chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            let mut embed = CreateEmbed::new()
                .title(if index == 0 {
                    format!("⚙️ Paramètres — {game_name} · {server_name}")
                } else {
                    "⚙️ Paramètres (suite)".to_string()
                })
                .field("Options de la partie", chunk, false)
                .color(0x2ecc71)
                .footer(CreateEmbedFooter::new(OPTIONS_FOOTER));
            if index == 0 {
                embed = embed.description("Réglages actuels de ce serveur.");
            }
            embed
        })
        .collect()
}

/// Extrait l'ID de serveur du topic d'un salon de session. Les salons
/// d'inscription et privés portent tous deux `... | session:{id} | ...` dans
/// leur topic (posé à la création). `None` si le salon n'est pas un salon de
/// session.
fn server_id_from_topic(topic: &str) -> Option<&str> {
    let after = topic.split("session:").nth(1)?;
    let id = after.split([' ', '|']).next()?.trim();
    (!id.is_empty()).then_some(id)
}

/// Construit les embeds de paramètres à afficher en réponse ÉPHÉMÈRE à la
/// commande `/game parametres`, à partir du salon d'où elle est lancée.
///
/// Contextuel : dans le salon privé des inscrits (topic `| private`) le mot de
/// passe est inclus (comme à la révélation) ; ailleurs (salon d'inscription) il
/// est masqué. Retourne un message d'erreur affichable si le salon n'est pas un
/// salon de session.
pub async fn params_embeds_for_channel(
    ctx: &Context,
    api: &ApiClient,
    channel: ChannelId,
) -> Result<Vec<CreateEmbed>, &'static str> {
    let topic = match channel.to_channel(&ctx.http).await {
        Ok(ch) => ch.guild().and_then(|g| g.topic),
        Err(_) => None,
    }
    .ok_or(
        "Utilise cette commande dans le salon d'un jeu (inscription ou salon privé des inscrits).",
    )?;

    let server_id =
        server_id_from_topic(&topic).ok_or("Ce salon n'est pas rattaché à un serveur de jeu.")?;
    let is_private = topic.contains("| private");

    let detail = api
        .get_game_server(server_id)
        .await
        .map_err(|_| "Serveur introuvable.")?;
    let game_name = api
        .get_game_template(&detail.server.template_id)
        .await
        .map(|t| t.name)
        .unwrap_or_else(|_| "Jeu".into());
    let options = if is_private {
        public_game_options(&detail.config)
    } else {
        registration_options(&detail.config)
    };
    Ok(build_options_embeds(
        &game_name,
        &detail.server.name,
        &options,
    ))
}

/// Decoupe les options en blocs tenant dans la VALEUR d'un champ d'embed
/// Discord, dont la limite dure est 1024 caracteres. On borne a 1000 pour
/// garder une marge (chaque option est deja une ligne markdown courte).
fn chunk_options(options: &[String]) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in options {
        if !current.is_empty() && current.len() + line.len() + 1 > 1000 {
            chunks.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn public_cover_url(path: Option<&str>) -> Option<String> {
    let path = path?.trim();
    if path.starts_with("https://") || path.starts_with("http://") {
        return Some(path.to_string());
    }
    let base = std::env::var("WEB_FRONT_URL").ok()?;
    let base = base.trim().trim_end_matches('/');
    if base.is_empty() {
        return None;
    }
    Some(format!("{base}/{}", path.trim_start_matches('/')))
}

async fn grant_session_role(
    ctx: &Context,
    component: &ComponentInteraction,
    server_id: &str,
    game_name: &str,
) {
    let Some(guild_id) = component.guild_id else {
        return;
    };
    let expected = session_role_name(game_name, server_id);
    let role_id = match guild_id.roles(&ctx.http).await {
        Ok(roles) => roles.values().find(|r| r.name == expected).map(|r| r.id),
        Err(e) => {
            tracing::warn!(error = %e, server_id, "game-portal: lecture roles impossible");
            None
        }
    };
    let Some(role_id) = role_id else {
        tracing::warn!(
            server_id,
            role = expected,
            "game-portal: role de session introuvable"
        );
        return;
    };
    match guild_id.member(&ctx.http, component.user.id).await {
        Ok(member) => {
            if let Err(e) = member.add_role(&ctx.http, role_id).await {
                tracing::warn!(error = %e, server_id, user = %component.user.id, "game-portal: attribution role de session impossible");
            }
        }
        Err(e) => tracing::warn!(error = %e, server_id, "game-portal: membre introuvable"),
    }
}

async fn on_reveal_ip_component(
    api: &ApiClient,
    ctx: &Context,
    component: &ComponentInteraction,
    server_id: &str,
) {
    // Defer ephemere IMMEDIAT : le demarrage du conteneur (allocation de ports,
    // reseau, pull d'image, start) depasse largement les 3 s d'ack imposees par
    // Discord. Sans ce defer, l'interaction echoue en « n'a pas repondu a
    // temps » avant meme la fin de l'appel API.
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, server_id, "game-portal: defer reveal-ip impossible");
        return;
    }

    let detail = match api.get_game_server(server_id).await {
        Ok(detail) => detail,
        Err(e) => {
            edit_deferred(ctx, component, format!("❌ Serveur introuvable : {e}")).await;
            return;
        }
    };
    if detail.server.owner_user_id != component.user.id.to_string() {
        edit_deferred(
            ctx,
            component,
            "⛔ Seul le propriétaire du serveur peut révéler son adresse.",
        )
        .await;
        return;
    }
    let outcome = match api
        .request_reveal_ip(server_id, &component.user.id.to_string())
        .await
    {
        Ok(outcome) => outcome,
        Err(e) => {
            edit_deferred(ctx, component, format!("❌ Ouverture impossible : {e}")).await;
            return;
        }
    };

    // Accuse ephemere au proprietaire.
    let minutes = outcome.delay_minutes;
    let debut = if outcome.started {
        "🚀 Le serveur démarre."
    } else {
        "🚀 Le serveur est déjà en ligne."
    };
    edit_deferred(
        ctx,
        component,
        format!(
            "{debut} L'adresse de connexion sera révélée dans le salon privé des inscrits dans **{minutes} minute(s)**."
        ),
    )
    .await;

    // Annonce publique dans le panneau d'inscription : tout le monde voit que la
    // session ouvre bientot. L'adresse, elle, ne parait qu'au salon prive a
    // l'echeance (publiee par le worker reveal-ip).
    let game_name = api
        .get_game_template(&detail.server.template_id)
        .await
        .map(|t| t.name)
        .unwrap_or_else(|_| "Le serveur".into());
    let _ = component
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .title(format!("⏳ {game_name} ouvre bientôt !"))
                    .description(format!(
                        "Le serveur démarre. L'adresse de connexion sera révélée dans le **salon privé des inscrits** dans **{minutes} minute(s)**.\n\nPas encore inscrit ? Clique sur **Je m'inscris** ci-dessus."
                    ))
                    .color(0x5865f2)
                    .footer(CreateEmbedFooter::new("Game Portal | Nexus"))
                    .timestamp(serenity::model::Timestamp::now()),
            ),
        )
        .await;
}

/// Edite la reponse ephemere DEJA deferee (voir le `Defer` en tete de
/// `on_reveal_ip_component`). A n'appeler qu'apres ce defer.
async fn edit_deferred(
    ctx: &Context,
    component: &ComponentInteraction,
    content: impl Into<String>,
) {
    let _ = component
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().content(content.into()),
        )
        .await;
}

// ── Panneau ──

pub fn panel_row(server_id: &str, ip_revealed: bool) -> CreateActionRow {
    let mut buttons = vec![CreateButton::new(format!("{REGISTER_PREFIX}{server_id}"))
        .label("Je m'inscris")
        .emoji('✅')
        .style(ButtonStyle::Success)];
    if !ip_revealed {
        buttons.push(
            CreateButton::new(format!("{REVEAL_IP_PREFIX}{server_id}"))
                .label("Révéler l'adresse IP")
                .emoji('🔓')
                .style(ButtonStyle::Danger),
        );
    }
    CreateActionRow::Buttons(buttons)
}

pub fn build_panel_embed(
    game_name: &str,
    server_name: &str,
    inscrits: &[String],
    ip_reveal_at: Option<&str>,
    ip_revealed: bool,
    public_host: Option<&str>,
    host_port: Option<u16>,
) -> CreateEmbed {
    let inscrits_txt = if inscrits.is_empty() {
        "_Personne pour l'instant — sois le premier !_".to_string()
    } else {
        inscrits
            .iter()
            .map(|u| format!("<@{u}>"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let ip_txt = if ip_revealed {
        match (public_host.filter(|h| !h.trim().is_empty()), host_port) {
            (Some(host), Some(p)) => format!("**Serveur ouvert !** `{host}:{p}`"),
            _ => "**Adresse indisponible : configuration incomplete.**".to_string(),
        }
    } else {
        match ip_reveal_at {
            Some(d) => format!("🔒 Masquee — revelee le **{}**", &d[..10.min(d.len())]),
            None => "🔒 Masquee".to_string(),
        }
    };

    CreateEmbed::new()
        .title(format!("🎮 {game_name} — {server_name}"))
        .description(
            "Un serveur de jeu est en preparation. Inscris-toi pour etre prevenu a l'ouverture !",
        )
        .field(
            format!("Inscrits ({})", inscrits.len()),
            inscrits_txt,
            false,
        )
        .field("Adresse (IP)", ip_txt, false)
        .field(
            "Réglages",
            "Tape `/game parametres` pour voir tous les réglages du serveur (réponse privée).",
            false,
        )
        .color(0x5865f2)
        .footer(CreateEmbedFooter::new("Game Portal | Nexus"))
        .timestamp(serenity::model::Timestamp::now())
}

/// Panneau du salon d'inscription : il indique l'ouverture, mais ne contient
/// jamais l'adresse, meme apres sa revelation dans le salon prive.
fn build_public_panel_embed(
    game_name: &str,
    server_name: &str,
    inscrits: &[String],
    ip_reveal_at: Option<&str>,
    ip_revealed: bool,
    cover_url: Option<&str>,
) -> CreateEmbed {
    let mut embed = build_panel_embed(
        game_name,
        server_name,
        inscrits,
        ip_reveal_at,
        ip_revealed,
        None,
        None,
    );
    if let Some(url) = cover_url {
        embed = embed.image(url);
    }
    embed
}

// ── Consumer d'evenements ──

/// Spawn le consumer durable de la stream Redis. Appele une fois au `ready`.
///
/// Sans REDIS_URL joignable, `listen_stream_group` boucle en reconnexion : les
/// salons ne sont simplement pas crees, le reste du bot fonctionne.
pub fn spawn(ctx: Context, api: Arc<ApiClient>) {
    tokio::spawn(async move {
        let consumer = crate::event_bus::default_consumer_name();
        crate::event_bus::listen_stream_group(
            "nexus-bot-game-portal".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                let api = api.clone();
                async move { handle_event(&ctx, &api, &payload_json).await }
            },
        )
        .await;
    });
}

async fn handle_event(ctx: &Context, api: &ApiClient, payload_json: &str) {
    let Ok(env) = serde_json::from_str::<serde_json::Value>(payload_json) else {
        return;
    };
    let event = env.get("event").and_then(|v| v.as_str());
    let data = env.get("data");
    let server_id = data
        .and_then(|d| d.get("server_id"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let guild_id = data
        .and_then(|d| d.get("guild_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok());

    let (Some(server_id), Some(guild_id)) = (server_id, guild_id) else {
        return;
    };

    use platform_core::nexus::ports::outbound::events::game_events as ev;
    match event {
        // Programmation ET demarrage creent les salons/panneau. La programmation
        // ouvre les inscriptions a l'avance ; le garde anti-doublon de
        // `on_started` evite qu'un demarrage ulterieur ne recree quoi que ce soit.
        Some(ev::SERVER_SCHEDULED) | Some(ev::SERVER_STARTED) => {
            on_started(ctx, api, GuildId::new(guild_id), &server_id).await
        }
        Some(ev::SERVER_STOPPED) => on_stopped(ctx, api, &server_id).await,
        Some(ev::SERVER_DELETED) => on_deleted(ctx, api, &server_id).await,
        Some(ev::IP_REVEAL) => on_ip_reveal(ctx, api, &server_id).await,
        Some(ev::DAILY_PING) => on_daily_ping(ctx, api, &server_id).await,
        _ => {}
    }
}

// ── Helpers partages ──

/// Resout le role Discord a pinguer pour le template d'un serveur.
///
/// Un role configure explicitement pour le template reste prioritaire. A
/// defaut, on reutilise le role deja cree par le module "Jeux
/// mentionnables" : d'abord par nom de template, puis par slug/base de slug
/// (`minecraft-vanilla` retrouve ainsi le jeu `Minecraft`).
async fn resolve_role(
    api: &ApiClient,
    guild_id: &str,
    slug: &str,
    game_name: &str,
) -> Option<RoleId> {
    let configured = api
        .list_template_settings(guild_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|s| s.template_slug == slug)
        .and_then(|s| s.discord_role_id)
        .and_then(|r| r.parse::<u64>().ok())
        .map(RoleId::new);
    if configured.is_some() {
        return configured;
    }

    let slug_base = slug.split(['-', '_']).next().unwrap_or(slug);
    for candidate in [game_name, slug_base, slug] {
        if let Ok(Some(game)) = api.get_game_by_name(guild_id, candidate).await {
            if let Some(role_id) = game
                .role_id
                .as_deref()
                .and_then(|role| role.parse::<u64>().ok())
                .map(RoleId::new)
            {
                return Some(role_id);
            }
        }
    }
    None
}

/// Nom lisible du jeu + role a pinguer, depuis le template du serveur.
async fn game_name_and_role(api: &ApiClient, server: &GameServer) -> (String, Option<RoleId>) {
    let template = api.get_game_template(&server.template_id).await.ok();
    let game_name = template
        .as_ref()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "Jeu".into());
    let role_id = match template.as_ref().map(|t| t.slug.clone()) {
        Some(slug) => resolve_role(api, &server.guild_id, &slug, &game_name).await,
        None => None,
    };
    (game_name, role_id)
}

fn parse_channel(id: Option<&String>) -> Option<ChannelId> {
    id.and_then(|s| s.parse::<u64>().ok()).map(ChannelId::new)
}

/// Nom de la categorie creee au premier demarrage si aucune n'est configuree.
const DEFAULT_SESSION_CATEGORY: &str = "Sessions de jeu";

/// Resout la categorie ou creer les salons de session, en la creant au besoin.
///
/// Trois etapes, de la moins couteuse a la plus couteuse :
///   1. `session_category_id` deja en config -> on verifie que la categorie
///      existe encore et qu'elle est bien de type Category ;
///   2. sinon, on adopte une categorie existante portant le nom attendu (cas
///      d'un admin qui l'a creee a la main) ;
///   3. sinon, on la cree.
///
/// Dans les cas 2 et 3, l'ID est PERSISTE via `set_config` : les demarrages
/// suivants sortent a l'etape 1 sans aucun appel Discord supplementaire.
/// C'est ce que sentinel-bot ne fait pas (`help_panel` recherche la categorie
/// par nom a chaque boot), d'ou une recreation si la categorie est renommee.
async fn ensure_session_category(
    ctx: &Context,
    api: &ApiClient,
    guild_id: GuildId,
    cfg: &std::collections::HashMap<String, String>,
) -> Option<ChannelId> {
    let guild_key = guild_id.to_string();

    // 1. Config existante — on ne fait confiance qu'apres verification.
    if let Some(id) = cfg
        .get("session_category_id")
        .and_then(|s| s.parse::<u64>().ok())
        .map(ChannelId::new)
    {
        match id.to_channel(&ctx).await {
            Ok(ch) => {
                if ch.guild().map(|g| g.kind) == Some(ChannelType::Category) {
                    return Some(id);
                }
                tracing::warn!(
                    %id,
                    "game-portal: session_category_id ne pointe pas sur une categorie -> resolution"
                );
            }
            Err(e) => {
                // Erreur reseau/rate limit : on garde la valeur configuree
                // plutot que de creer une categorie en double.
                if !is_not_found(&e) {
                    tracing::warn!(error = %e, %id, "game-portal: verification categorie impossible");
                    return Some(id);
                }
                tracing::warn!(%id, "game-portal: categorie de session disparue -> recreation");
            }
        }
    }

    // 2. Adoption d'une categorie existante portant le nom attendu.
    if let Ok(channels) = guild_id.channels(&ctx.http).await {
        if let Some(ch) = channels
            .values()
            .find(|c| c.kind == ChannelType::Category && c.name == DEFAULT_SESSION_CATEGORY)
        {
            persist_category(api, &guild_key, ch.id).await;
            return Some(ch.id);
        }
    }

    // 3. Creation.
    let created = guild_id
        .create_channel(
            &ctx.http,
            CreateChannel::new(DEFAULT_SESSION_CATEGORY).kind(ChannelType::Category),
        )
        .await;
    match created {
        Ok(ch) => {
            tracing::info!(guild = %guild_id, category = %ch.id, "game-portal: categorie de sessions creee");
            persist_category(api, &guild_key, ch.id).await;
            Some(ch.id)
        }
        Err(e) => {
            // Sans categorie les salons sont crees a la racine : degrade mais
            // fonctionnel, on ne bloque pas l'ouverture de session.
            tracing::warn!(error = %e, guild = %guild_id, "game-portal: creation de la categorie impossible");
            None
        }
    }
}

async fn persist_category(api: &ApiClient, guild_id: &str, category: ChannelId) {
    if let Err(e) = api
        .set_config(
            guild_id,
            MODULE_BOT_NAME,
            "session_category_id",
            &category.to_string(),
        )
        .await
    {
        // Non bloquant : la categorie sera re-resolue par son nom au prochain
        // demarrage (etape 2), simplement moins efficacement.
        tracing::warn!(error = %e, guild_id, "game-portal: memorisation de la categorie impossible");
    }
}

/// L'erreur serenity correspond-elle a un 404 Discord ?
fn is_not_found(e: &serenity::Error) -> bool {
    matches!(
        e,
        serenity::Error::Http(serenity::http::HttpError::UnsuccessfulRequest(res))
            if res.status_code == serenity::http::StatusCode::NOT_FOUND
    )
}

/// Le salon existe-t-il encore cote Discord ?
///
/// Passe par le cache puis l'API HTTP. En cas d'erreur autre qu'un 404 (panne
/// reseau, rate limit), on repond `true` : mieux vaut ne rien faire que
/// recreer des salons en double sur une erreur transitoire.
async fn channel_exists(ctx: &Context, channel_id: ChannelId) -> bool {
    match channel_id.to_channel(&ctx).await {
        Ok(_) => true,
        Err(e) if is_not_found(&e) => false,
        Err(e) => {
            tracing::warn!(error = %e, %channel_id, "game-portal: verification du salon impossible");
            true
        }
    }
}

/// Salon prive : @everyone ne voit rien, le role du jeu voit et participe.
///
/// Les permissions accordees dependent du TYPE de salon. Discord refuse une
/// creation (50013) quand l'overwrite accorde une permission que le bot ne
/// possede pas lui-meme : demander CONNECT et SPEAK sur un salon textuel, ou
/// SEND_MESSAGES sur un vocal, expose a un echec pour une permission dont le
/// salon n'a de toute facon aucun usage.
fn build_overwrites(
    guild_id: GuildId,
    role_id: Option<RoleId>,
    kind: ChannelType,
) -> Vec<PermissionOverwrite> {
    // @everyone porte le meme ID que la guild.
    let mut ows = vec![PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::VIEW_CHANNEL,
        kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
    }];
    if let Some(rid) = role_id {
        let specifiques = if kind == ChannelType::Voice {
            Permissions::CONNECT | Permissions::SPEAK
        } else {
            Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY
        };
        ows.push(PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL | specifiques,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Role(rid),
        });
    }
    ows
}

async fn create_channel(
    ctx: &Context,
    guild_id: GuildId,
    name: &str,
    kind: ChannelType,
    category: Option<ChannelId>,
    overwrites: &[PermissionOverwrite],
    topic: Option<&str>,
) -> Option<ChannelId> {
    let construire = |cat: Option<ChannelId>| {
        let mut b = CreateChannel::new(name)
            .kind(kind)
            .permissions(overwrites.to_vec());
        if let Some(c) = cat {
            b = b.category(c);
        }
        if let Some(topic) = topic {
            b = b.topic(topic);
        }
        b
    };

    let premiere = match guild_id
        .create_channel(&ctx.http, construire(category))
        .await
    {
        Ok(ch) => return Some(ch.id),
        Err(e) => e,
    };

    // Une categorie Discord plafonne a 50 salons. Le vocal etant cree apres le
    // textuel, c'est lui qui bute en premier sur la limite — le textuel passe
    // et le vocal manque, sans que rien ne le signale.
    //
    // Plutot que d'abandonner la session, on recree hors categorie : le salon
    // est moins bien range mais il existe, ce qui est preferable a une session
    // muette. Le log dit pourquoi.
    if category.is_some() {
        tracing::warn!(
            error = %premiere,
            name,
            ?kind,
            "game-portal: echec creation salon dans la categorie -> nouvel essai hors categorie"
        );
        match guild_id.create_channel(&ctx.http, construire(None)).await {
            Ok(ch) => return Some(ch.id),
            Err(e) => {
                tracing::error!(
                    error = %e,
                    name,
                    ?kind,
                    "game-portal: echec creation salon, y compris hors categorie"
                );
                return None;
            }
        }
    }

    tracing::error!(error = %premiere, name, ?kind, "game-portal: echec creation salon");
    None
}

/// Nom de salon Discord valide : minuscules, tirets, sans accents.
fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_end_matches('-').to_string();
    if trimmed.is_empty() {
        "serveur".to_string()
    } else {
        trimmed
    }
}

// ── Demarrage d'un serveur -> creation des salons ──

async fn on_started(ctx: &Context, api: &ApiClient, guild_id: GuildId, server_id: &str) {
    let detail = match api.get_game_server(server_id).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(error = %e, server_id, "game-portal: echec lecture serveur");
            return;
        }
    };
    let server = detail.server;
    // Salons deja enregistres : soit l'evenement est rejoue (salon bien vivant,
    // rien a faire), soit le salon a disparu cote Discord et la base ment.
    //
    // On VERIFIE plutot que de faire confiance : un salon peut avoir ete
    // supprime par un wipe de guilde (module guild_backup de sentinel-bot),
    // par un admin a la main, ou par n'importe quel futur nettoyage. Sans cette
    // verification, la garde ci-dessous bloquerait la recreation pour toujours
    // et le game-portal resterait casse en silence.
    if let Some(existing) = parse_channel(server.text_channel_id.as_ref()) {
        if channel_exists(ctx, existing).await {
            return;
        }
        // Salon fantome : on libere les references avant de recreer, sinon le
        // claim `set_session_channels` plus bas refuserait (garde anti-doublon).
        tracing::warn!(
            server_id,
            channel_id = %existing,
            "game-portal: salon de session disparu cote Discord -> recreation"
        );
        if let Err(e) = api.set_session_channels(server_id, None, None).await {
            tracing::warn!(error = %e, server_id, "game-portal: echec liberation des salons fantomes");
            return;
        }
    }

    let (game_name, role_id) = game_name_and_role(api, &server).await;
    let cover_url = api
        .get_game_template(&server.template_id)
        .await
        .ok()
        .and_then(|template| public_cover_url(template.cover_image_url.as_deref()));

    let cfg = api
        .get_guild_config(&server.guild_id, MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let category = ensure_session_category(ctx, api, guild_id, &cfg).await;

    let session_role = match guild_id
        .create_role(
            &ctx.http,
            EditRole::new()
                .name(session_role_name(&game_name, server_id))
                .colour(Colour::new(0x5865f2))
                .mentionable(false)
                .hoist(false),
        )
        .await
    {
        Ok(role) => role,
        Err(e) => {
            tracing::error!(error = %e, server_id, "game-portal: creation role de session impossible");
            return;
        }
    };

    let text_ch = create_channel(
        ctx,
        guild_id,
        &registration_channel_name(&game_name),
        ChannelType::Text,
        category,
        &build_overwrites(guild_id, role_id, ChannelType::Text),
        Some(&format!(
            "Nexus Game Portal | session:{server_id} | registration"
        )),
    )
    .await;
    let private_text_ch = create_channel(
        ctx,
        guild_id,
        &private_text_name(&game_name),
        ChannelType::Text,
        category,
        &build_overwrites(guild_id, Some(session_role.id), ChannelType::Text),
        Some(&private_text_topic(server_id)),
    )
    .await;
    let voice_ch = create_channel(
        ctx,
        guild_id,
        &format!("Vocal {}", server.name),
        ChannelType::Voice,
        category,
        &build_overwrites(guild_id, Some(session_role.id), ChannelType::Voice),
        None,
    )
    .await;

    let (Some(text_ch), Some(private_text_ch), Some(voice_ch)) =
        (text_ch, private_text_ch, voice_ch)
    else {
        if let Some(ch) = text_ch {
            let _ = ch.delete(&ctx.http).await;
        }
        if let Some(ch) = private_text_ch {
            let _ = ch.delete(&ctx.http).await;
        }
        if let Some(ch) = voice_ch {
            let _ = ch.delete(&ctx.http).await;
        }
        let _ = guild_id.delete_role(&ctx.http, session_role.id).await;
        return;
    };

    // Enregistrement cote API : le claim sert de garde anti-doublon. Si le
    // claim echoue (claimed=false), des salons etaient deja enregistres
    // (evenement rejoue) -> on supprime ceux qu'on vient de creer. Une erreur
    // reseau laisse les salons en place (pas de suppression a tort).
    match api
        .set_session_channels(
            server_id,
            Some(&text_ch.to_string()),
            Some(&voice_ch.to_string()),
        )
        .await
    {
        Ok(false) => {
            let _ = text_ch.delete(&ctx.http).await;
            let _ = private_text_ch.delete(&ctx.http).await;
            let _ = voice_ch.delete(&ctx.http).await;
            let _ = guild_id.delete_role(&ctx.http, session_role.id).await;
            tracing::warn!(
                server_id,
                "game-portal: salons deja enregistres (evenement rejoue) -> doublons supprimes"
            );
            return;
        }
        Ok(true) => {}
        Err(e) => {
            tracing::warn!(error = %e, server_id, "game-portal: echec enregistrement salons (salons conserves)");
        }
    }

    let registered_user_ids: Vec<String> = api
        .list_server_registrations(server_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.user_id)
        .collect();
    for user_id in &registered_user_ids {
        if let Ok(user_num) = user_id.parse::<u64>() {
            if let Ok(member) = guild_id.member(&ctx.http, user_num).await {
                let _ = member.add_role(&ctx.http, session_role.id).await;
            }
        }
    }

    let embed = build_public_panel_embed(
        &game_name,
        &server.name,
        &registered_user_ids,
        server.ip_reveal_at.as_deref(),
        server.ip_revealed,
        cover_url.as_deref(),
    );
    let msg = text_ch
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .embed(embed)
                .components(vec![panel_row(server_id, server.ip_revealed)]),
        )
        .await;
    if let Ok(m) = &msg {
        let _ = text_ch.pin(&ctx.http, m.id).await;
    }

    if let Some(rid) = role_id {
        let _ = text_ch
            .send_message(
                &ctx.http,
                CreateMessage::new().content(format!(
                    "<@&{rid}> un serveur **{game_name}** ouvre bientot ! Inscris-toi ci-dessus."
                )),
            )
            .await;
    }

    tracing::info!(guild = %guild_id, server_id, "game-portal: session ouverte (salons crees)");
}

// ── Arret -> on CONSERVE les salons ──

/// Serveur arrete (mais pas supprime) : les salons de session, le role et le
/// panneau d'inscription sont CONSERVES. Un arret est temporaire — on veut
/// pouvoir redemarrer sans reconstruire salons ni inscriptions, et sans que le
/// role de session (donc l'acces des inscrits) ne saute a chaque pause. La
/// suppression effective des salons n'a lieu qu'a la suppression du jeu
/// (`on_deleted`, evenement `game_server_deleted`).
async fn on_stopped(_ctx: &Context, _api: &ApiClient, server_id: &str) {
    tracing::info!(server_id, "game-portal: session arretee (salons conserves)");
}

// ── Suppression du jeu -> suppression des salons ──

async fn on_deleted(ctx: &Context, api: &ApiClient, server_id: &str) {
    let Ok(detail) = api.get_game_server(server_id).await else {
        return;
    };
    for ch in [
        parse_channel(detail.server.text_channel_id.as_ref()),
        parse_channel(detail.server.voice_channel_id.as_ref()),
    ]
    .into_iter()
    .flatten()
    {
        let _ = ch.delete(&ctx.http).await;
    }

    if let Ok(guild_num) = detail.server.guild_id.parse::<u64>() {
        let guild_id = GuildId::new(guild_num);
        let game_name = api
            .get_game_template(&detail.server.template_id)
            .await
            .map(|t| t.name)
            .unwrap_or_else(|_| "jeu".into());
        let private_name = private_text_name(&game_name);
        let legacy_private_name = legacy_private_text_name(server_id);
        let private_topic = private_text_topic(server_id);
        if let Ok(channels) = guild_id.channels(&ctx.http).await {
            for channel in channels.values().filter(|c| {
                (c.name == private_name && c.topic.as_deref() == Some(private_topic.as_str()))
                    || c.name == legacy_private_name
            }) {
                let _ = channel.delete(&ctx.http).await;
            }
        }
        let role_name = session_role_name(&game_name, server_id);
        if let Ok(roles) = guild_id.roles(&ctx.http).await {
            for role in roles.values().filter(|r| r.name == role_name) {
                let _ = guild_id.delete_role(&ctx.http, role.id).await;
            }
        }
    }
    // Libere les salons cote API (sinon un futur demarrage se croirait rejoue).
    if let Err(e) = api.set_session_channels(server_id, None, None).await {
        tracing::warn!(error = %e, server_id, "game-portal: echec liberation des salons");
    }
    tracing::info!(server_id, "game-portal: session fermee (salons supprimes)");
}

// ── Revelation d'IP ──

async fn on_ip_reveal(ctx: &Context, api: &ApiClient, server_id: &str) {
    let Ok(detail) = api.get_game_server(server_id).await else {
        return;
    };
    let server = detail.server;
    let Some(text_ch) = parse_channel(server.text_channel_id.as_ref()) else {
        return;
    };

    let template = api.get_game_template(&server.template_id).await.ok();
    let game_name = template
        .as_ref()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "Jeu".into());
    let cover_url = public_cover_url(
        template
            .as_ref()
            .and_then(|template| template.cover_image_url.as_deref()),
    );

    // Publie l'adresse uniquement dans le salon textuel prive des inscrits.
    if let Ok(guild_num) = server.guild_id.parse::<u64>() {
        let guild_id = GuildId::new(guild_num);
        let private_name = private_text_name(&game_name);
        let legacy_private_name = legacy_private_text_name(server_id);
        let private_topic = private_text_topic(server_id);
        if let Ok(channels) = guild_id.channels(&ctx.http).await {
            if let Some(private_ch) = channels.values().find(|c| {
                (c.name == private_name && c.topic.as_deref() == Some(private_topic.as_str()))
                    || c.name == legacy_private_name
            }) {
                let address = match (
                    server
                        .public_host
                        .as_deref()
                        .filter(|h| !h.trim().is_empty()),
                    server.host_port,
                ) {
                    (Some(host), Some(port)) => format!("`{host}:{port}`"),
                    _ => "_Adresse indisponible, contacte le propriétaire._".to_string(),
                };
                let mut card = CreateEmbed::new()
                    .title(format!("🎮 {game_name} — {name}", name = server.name))
                    .description(format!(
                        "🔓 **Serveur ouvert**\nConnexion : {address}\n\nTape `/game parametres` ici pour voir tous les réglages (mot de passe inclus)."
                    ))
                    .color(0x5865f2)
                    .footer(CreateEmbedFooter::new("Game Portal | Accès privé"))
                    .timestamp(serenity::model::Timestamp::now());
                if let Some(url) = cover_url.as_deref() {
                    card = card.image(url);
                }
                let _ = private_ch
                    .send_message(&ctx.http, CreateMessage::new().embed(card))
                    .await;
            }
        }
    }

    // Le panneau public indique seulement que le serveur est ouvert.
    let user_ids: Vec<String> = api
        .list_server_registrations(server_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.user_id)
        .collect();
    let embed = build_public_panel_embed(
        &game_name,
        &server.name,
        &user_ids,
        None,
        true,
        cover_url.as_deref(),
    );
    if let Ok(pins) = text_ch.pins(&ctx.http).await {
        if let Some(m) = pins.into_iter().find(|m| !m.embeds.is_empty()) {
            let _ = text_ch
                .edit_message(
                    &ctx.http,
                    m.id,
                    EditMessage::new()
                        .embed(embed)
                        .components(vec![panel_row(server_id, true)]),
                )
                .await;
        }
    }

    tracing::info!(server_id, "game-portal: IP revelee");
}

// ── Ping quotidien ──

async fn on_daily_ping(ctx: &Context, api: &ApiClient, server_id: &str) {
    let Ok(detail) = api.get_game_server(server_id).await else {
        return;
    };
    let server = detail.server;
    let Some(text_ch) = parse_channel(server.text_channel_id.as_ref()) else {
        return;
    };
    let (game_name, role_id) = game_name_and_role(api, &server).await;
    let Some(rid) = role_id else { return };

    // Jours restants avant la revelation.
    let remaining = server.ip_reveal_at.as_deref().and_then(|d| {
        chrono::DateTime::parse_from_rfc3339(d).ok().map(|dt| {
            (dt.with_timezone(&chrono::Utc) - chrono::Utc::now())
                .num_days()
                .max(0)
        })
    });
    let when = match remaining {
        Some(0) => "aujourd hui".to_string(),
        Some(n) => format!("dans **{n}** jour(s)"),
        None => "bientot".to_string(),
    };

    let _ = text_ch
        .send_message(
            &ctx.http,
            CreateMessage::new().content(format!(
                "<@&{rid}> Le serveur **{game_name}** ouvre {when} ! Inscris-toi sur le panneau."
            )),
        )
        .await;
}

#[cfg(test)]
mod tests {
    use super::{private_text_name, registration_channel_name};

    #[test]
    fn session_channels_use_the_game_name() {
        assert_eq!(
            registration_channel_name("7 Days to Die"),
            "inscription-7-days-to-die"
        );
        assert_eq!(private_text_name("7 Days to Die"), "salon-7-days-to-die");
    }
}
