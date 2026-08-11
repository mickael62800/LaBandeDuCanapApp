//! Module Game Portal (bot) — cycle de vie des salons de session.
//!
//! Porte depuis sentinel-bot (modules/game_portal). Pilote par les evenements
//! publies par nexus-api sur la stream Redis `nexus:events` :
//!   - `game_server_started` : cree un salon texte + un salon vocal PRIVES
//!     (visibles du seul role du jeu) dans la categorie configuree, epingle un
//!     panneau avec bouton d'inscription et ping le role ;
//!   - `game_server_stopped` / `game_server_deleted` : supprime les salons ;
//!   - `game_ip_reveal` : poste l'adresse de connexion et rafraichit le panneau ;
//!   - `game_daily_ping` : rappelle l'ouverture a venir au role du jeu.
//!
//! La configuration par guild (categorie, hote public) est lue via
//! `GET /api/config/{guild_id}/game-portal`.

use std::sync::Arc;

use serenity::all::{
    ButtonStyle, ChannelId, ChannelType, ComponentInteraction, Context, CreateActionRow,
    CreateButton, CreateChannel, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditMessage, GuildId, PermissionOverwrite,
    PermissionOverwriteType, Permissions, RoleId,
};

use crate::api_client::{ApiClient, GameServer};

/// Nom du module dans `bot_guild_config` (cle de lecture de la config guild).
const MODULE_BOT_NAME: &str = "game-portal";

/// custom_id du bouton d'inscription : `gp_register:{server_id}`.
pub const REGISTER_PREFIX: &str = "gp_register:";

pub fn handles_component(custom_id: &str) -> bool {
    custom_id.starts_with(REGISTER_PREFIX)
}

pub async fn on_component(api: &ApiClient, ctx: &Context, component: &ComponentInteraction) {
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
        let game_name = api
            .get_game_template(&detail.server.template_id)
            .await
            .map(|t| t.name)
            .unwrap_or_else(|_| "Jeu".into());
        let embed = build_panel_embed(
            &game_name,
            &detail.server.name,
            &user_ids,
            detail.server.ip_reveal_at.as_deref(),
            detail.server.ip_revealed,
            detail.server.host_port,
        );
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(vec![register_row(server_id)]),
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

// ── Panneau ──

pub fn register_row(server_id: &str) -> CreateActionRow {
    CreateActionRow::Buttons(vec![CreateButton::new(format!(
        "{REGISTER_PREFIX}{server_id}"
    ))
    .label("Je m'inscris")
    .emoji('✅')
    .style(ButtonStyle::Success)])
}

pub fn build_panel_embed(
    game_name: &str,
    server_name: &str,
    inscrits: &[String],
    ip_reveal_at: Option<&str>,
    ip_revealed: bool,
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
        match host_port {
            Some(p) => format!("**Serveur ouvert !** Port : `{p}`"),
            None => "**Serveur ouvert !**".to_string(),
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
        .color(0x5865f2)
        .footer(CreateEmbedFooter::new("Game Portal | Nexus"))
        .timestamp(serenity::model::Timestamp::now())
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

    use nexus_core::ports::outbound::events::game_events as ev;
    match event {
        Some(ev::SERVER_STARTED) => on_started(ctx, api, GuildId::new(guild_id), &server_id).await,
        Some(ev::SERVER_STOPPED) | Some(ev::SERVER_DELETED) => {
            on_stopped(ctx, api, &server_id).await
        }
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
) -> Option<ChannelId> {
    let construire = |cat: Option<ChannelId>| {
        let mut b = CreateChannel::new(name)
            .kind(kind)
            .permissions(overwrites.to_vec());
        if let Some(c) = cat {
            b = b.category(c);
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

    let cfg = api
        .get_guild_config(&server.guild_id, MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let category = ensure_session_category(ctx, api, guild_id, &cfg).await;

    let text_ch = create_channel(
        ctx,
        guild_id,
        &format!("game-{}", slugify(&server.name)),
        ChannelType::Text,
        category,
        &build_overwrites(guild_id, role_id, ChannelType::Text),
    )
    .await;
    let voice_ch = create_channel(
        ctx,
        guild_id,
        &format!("Vocal {}", server.name),
        ChannelType::Voice,
        category,
        &build_overwrites(guild_id, role_id, ChannelType::Voice),
    )
    .await;

    let Some(text_ch) = text_ch else { return };

    // Enregistrement cote API : le claim sert de garde anti-doublon. Si le
    // claim echoue (claimed=false), des salons etaient deja enregistres
    // (evenement rejoue) -> on supprime ceux qu'on vient de creer. Une erreur
    // reseau laisse les salons en place (pas de suppression a tort).
    match api
        .set_session_channels(
            server_id,
            Some(&text_ch.to_string()),
            voice_ch.map(|c| c.to_string()).as_deref(),
        )
        .await
    {
        Ok(false) => {
            let _ = text_ch.delete(&ctx.http).await;
            if let Some(vc) = voice_ch {
                let _ = vc.delete(&ctx.http).await;
            }
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

    let embed = build_panel_embed(
        &game_name,
        &server.name,
        &[],
        server.ip_reveal_at.as_deref(),
        server.ip_revealed,
        server.host_port,
    );
    let msg = text_ch
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .embed(embed)
                .components(vec![register_row(server_id)]),
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

// ── Arret / suppression -> suppression des salons ──

async fn on_stopped(ctx: &Context, api: &ApiClient, server_id: &str) {
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

    let (game_name, role_id) = game_name_and_role(api, &server).await;

    // Adresse : {hote public}:{port} si l'hote est configure, sinon le port seul.
    let cfg = api
        .get_guild_config(&server.guild_id, MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let host = cfg.get("session_public_host").cloned().unwrap_or_default();
    let addr = match (host.trim().is_empty(), server.host_port) {
        (false, Some(p)) => format!("`{}:{}`", host.trim(), p),
        (true, Some(p)) => format!("port `{p}`"),
        _ => "_communiquee par le staff_".to_string(),
    };

    let ping = role_id.map(|r| format!("<@&{r}> ")).unwrap_or_default();
    let _ = text_ch
        .send_message(
            &ctx.http,
            CreateMessage::new().content(format!(
                "{ping}Le serveur **{game_name}** est **OUVERT** ! Connexion : {addr}"
            )),
        )
        .await;

    // Rafraichit le panneau epingle (IP desormais visible).
    let user_ids: Vec<String> = api
        .list_server_registrations(server_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.user_id)
        .collect();
    let embed = build_panel_embed(
        &game_name,
        &server.name,
        &user_ids,
        None,
        true,
        server.host_port,
    );
    if let Ok(pins) = text_ch.pins(&ctx.http).await {
        if let Some(m) = pins.into_iter().find(|m| !m.embeds.is_empty()) {
            let _ = text_ch
                .edit_message(
                    &ctx.http,
                    m.id,
                    EditMessage::new()
                        .embed(embed)
                        .components(vec![register_row(server_id)]),
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
