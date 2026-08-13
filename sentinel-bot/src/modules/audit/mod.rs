//! Module audit — logging evenements Discord (ex audit-bot).
//!
//! Sous-modules :
//! - `api_client` : AuditEvent + ApiClient (send, search, watched users)
//! - `audit_event` : builder fluent pour AuditEvent
//! - `message_cache` : cache LRU des messages (pour retrouver les deletes)
//! - `anomaly` : detection d'anomalies (mass ban/delete/role)
//! - `permission_diff` : diff lisible de permissions Discord
//! - `watched_users` : bootstrap + refresh cache utilisateurs surveilles
//! - `handlers` : sous-handlers par type d'event (message, member, ...)
//! - `commands` : slash commands (/audit search, /audit stats)

pub const MODULE_BOT_NAME: &str = "audit-bot";

pub mod anomaly;
pub mod api_client;
pub mod audit_event;
pub mod commands;
pub mod handlers;
pub mod message_cache;
pub mod permission_diff;
pub mod role_card;
pub mod watched_users;

use std::sync::Arc;

use dashmap::DashSet;
use serenity::all::{CommandInteraction, CreateCommand};
use serenity::model::channel::Message;
use serenity::model::event::MessageUpdateEvent;
use serenity::model::guild::{Guild, Member, Role};
use serenity::model::id::{ChannelId, GuildId, MessageId, RoleId};
use serenity::model::user::User;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::discord_helpers::is_module_enabled;
use crate::shared::heartbeat::ApiClientKey;

use api_client::{ApiClient, AuditEvent};

// ── TypeMapKey definitions (merged from handler/type_keys.rs) ──

pub struct MessageCacheKey;
impl TypeMapKey for MessageCacheKey {
    type Value = message_cache::MessageCache;
}

pub struct ConfigKey;
impl TypeMapKey for ConfigKey {
    type Value = AuditConfig;
}

/// Cache des user_ids surveilles (rafraichi toutes les 60s).
pub struct WatchedUserIdsKey;
impl TypeMapKey for WatchedUserIdsKey {
    type Value = Arc<DashSet<String>>;
}

/// Config audit lue depuis l'environnement (subset de l'ancienne Config).
#[derive(Clone)]
pub struct AuditConfig {
    pub message_cache_size: usize,
    pub anomaly_window_secs: u64,
    pub anomaly_mass_ban_threshold: usize,
    pub anomaly_mass_delete_threshold: usize,
    pub anomaly_mass_role_threshold: usize,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            message_cache_size: crate::shared::config::load_env("MESSAGE_CACHE_SIZE", 10_000)
                .clamp(100, 100_000),
            anomaly_window_secs: crate::shared::config::load_env("ANOMALY_WINDOW_SECS", 60),
            anomaly_mass_ban_threshold: crate::shared::config::load_env("ANOMALY_MASS_BAN", 5),
            anomaly_mass_delete_threshold: crate::shared::config::load_env(
                "ANOMALY_MASS_DELETE",
                20,
            ),
            anomaly_mass_role_threshold: crate::shared::config::load_env("ANOMALY_MASS_ROLE", 10),
        }
    }
}

// ── Init TypeMapKeys ──

/// Insere les TypeMapKeys du module audit.
pub fn init_typemap(data: &mut serenity::prelude::TypeMap) {
    let audit_config = AuditConfig::default();
    info!(
        max_messages_per_guild = audit_config.message_cache_size,
        "Cache memoire des messages initialise"
    );
    data.insert::<MessageCacheKey>(message_cache::MessageCache::new(
        audit_config.message_cache_size,
    ));
    data.insert::<ConfigKey>(audit_config);
    data.insert::<WatchedUserIdsKey>(Arc::new(DashSet::new()));
    data.insert::<role_card::RoleCardTrackerKey>(Arc::new(role_card::RoleCardTracker::default()));
}

// ── Commands registration ──

pub fn register_commands() -> Vec<CreateCommand> {
    commands::all()
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if let Some(guild_id) = command.guild_id {
        if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
            return;
        }
    }

    commands::audit::handle(ctx, command).await;
}

// ── Helper functions (extracted from impl Handler) ──

/// Envoie un evenement d'audit a l'API.
pub async fn send_event(ctx: &Context, event: AuditEvent) {
    let data = ctx.data.read().await;
    if let Some(base) = data.get::<crate::shared::grpc_client::GrpcClientKey>() {
        let api = ApiClient::new(base.clone());
        if let Err(e) = api.send_audit_event(&event).await {
            warn!(error = %e, event_type = %event.event_type, "Erreur envoi audit event");
        }
    }
}

/// Charge les seuils anomaly per-guild depuis bot_guild_config. Si une cle
/// est absente, fallback sur le default global (env-driven).
pub async fn anomaly_thresholds_for(ctx: &Context, guild_id: &str) -> anomaly::AnomalyThresholds {
    let default = {
        let data = ctx.data.read().await;
        data.get::<ConfigKey>()
            .map(|c| anomaly::AnomalyThresholds {
                mass_ban: c.anomaly_mass_ban_threshold,
                mass_delete: c.anomaly_mass_delete_threshold,
                mass_role_change: c.anomaly_mass_role_threshold,
            })
            .unwrap_or_default()
    };
    let cfg = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(api) => api
                .get_guild_config_for(guild_id, MODULE_BOT_NAME)
                .await
                .unwrap_or_default(),
            None => return default,
        }
    };
    anomaly::AnomalyThresholds {
        mass_ban: cfg
            .get("anomaly_mass_ban_threshold")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.mass_ban),
        mass_delete: cfg
            .get("anomaly_mass_delete_threshold")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.mass_delete),
        mass_role_change: cfg
            .get("anomaly_mass_role_threshold")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.mass_role_change),
    }
}

/// Envoie un evenement de moderation a l'API pour DECISION d'anomalie.
/// L'API agrege sur sa fenetre glissante serveur, applique les seuils
/// (resolus per-guild ici) et renvoie une alerte a afficher le cas echeant.
/// Le bot ne decide plus : il ne fait que relayer l'event et afficher.
///
/// `category` : "ban" | "kick" | "delete" | "role_change".
/// `increment` : nombre d'evenements (> 1 pour une purge bulk).
pub async fn detect_anomaly(
    ctx: &Context,
    guild_id: &str,
    category: &str,
    increment: usize,
) -> Option<anomaly::AnomalyAlert> {
    let thresholds = anomaly_thresholds_for(ctx, guild_id).await;
    let (base, window_secs) = {
        let data = ctx.data.read().await;
        let base = data
            .get::<crate::shared::grpc_client::GrpcClientKey>()?
            .clone();
        let window_secs = data
            .get::<ConfigKey>()
            .map(|c| c.anomaly_window_secs)
            .unwrap_or(60);
        (base, window_secs)
    };
    let api = ApiClient::new(base);
    match api
        .detect_moderation_anomaly(guild_id, category, increment, window_secs, &thresholds)
        .await
    {
        Ok(alert) => alert,
        Err(e) => {
            warn!(error = %e, category = %category, "Erreur detection anomalie via API");
            None
        }
    }
}

/// Pousse un log structure dans la queue d'envoi de l'API client.
pub async fn log(ctx: &Context, level: &str, guild_id: &str, message: &str) {
    let data = ctx.data.read().await;
    if let Some(api) = data.get::<ApiClientKey>() {
        api.send_log(level, guild_id, message);
    }
}

/// Poste un embed dans le premier salon configure trouve parmi `config_keys`,
/// avec fallback sur `log_channel_id`. Si aucun n'est configure, ne fait rien.
pub async fn post_to_channel(
    ctx: &Context,
    guild_id: &str,
    config_keys: &[&str],
    embed: serenity::builder::CreateEmbed,
) {
    // Lire la config une fois
    let config = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(base) => base
                .get_guild_config_for(guild_id, MODULE_BOT_NAME)
                .await
                .unwrap_or_default(),
            None => return,
        }
    };

    // Chercher le premier salon configure parmi les cles fournies,
    // puis fallback sur log_channel_id
    let mut channel_id: Option<u64> = None;
    for key in config_keys {
        if let Some(v) = config.get(*key) {
            if let Ok(id) = v.parse::<u64>() {
                if id > 0 {
                    channel_id = Some(id);
                    break;
                }
            }
        }
    }
    if channel_id.is_none() {
        if let Some(v) = config.get("log_channel_id") {
            if let Ok(id) = v.parse::<u64>() {
                if id > 0 {
                    channel_id = Some(id);
                }
            }
        }
    }

    let channel = match channel_id {
        Some(id) => ChannelId::new(id),
        None => return,
    };

    let msg = serenity::builder::CreateMessage::new().embed(embed);
    if let Err(e) = channel.send_message(&ctx.http, msg).await {
        warn!(error = %e, channel_id = %channel, "Echec envoi embed audit");
    }
}

/// Log une-ligne d'utilisation d'une commande admin/moderateur, dans un salon
/// DEDIE et parametrable (`command_log_channel_id`). Ne poste QUE si le module
/// est actif, que `command_log_enabled` vaut true et que le salon est configure
/// (aucun fallback sur le log d'audit general, pour ne pas le polluer).
pub async fn log_admin_command(
    ctx: &Context,
    guild_id: &str,
    user_id: &str,
    user_name: &str,
    full_command: &str,
    reason: Option<&str>,
) {
    // Persistance AVANT le postage Discord, et independamment de lui : le
    // salon est optionnel et destine a disparaitre (les logs migrent vers le
    // web), alors qu'une commande admin doit rester tracable. C'etait le seul
    // evenement poste dans Discord sans aucune ecriture en base.
    send_event(
        ctx,
        AuditEvent {
            guild_id: guild_id.to_string(),
            event_type: "admin_command".to_string(),
            actor_id: Some(user_id.to_string()),
            actor_name: Some(user_name.to_string()),
            target_id: None,
            target_name: None,
            channel_id: None,
            channel_name: None,
            details: serde_json::json!({
                "command": full_command,
                "reason": reason.map(str::trim).filter(|r| !r.is_empty()),
            }),
        },
    )
    .await;

    let config = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(base) => base
                .get_guild_config_for(guild_id, MODULE_BOT_NAME)
                .await
                .unwrap_or_default(),
            None => return,
        }
    };

    // Opt-in explicite : toggle + salon dedie.
    let enabled = config
        .get("command_log_enabled")
        .map(|v| {
            platform_core::sentinel::domain::entities::system::config_parsers::parse_bool_str(v)
        })
        .unwrap_or(false);
    if !enabled {
        return;
    }
    let channel_id = match config
        .get("command_log_channel_id")
        .and_then(|v| v.parse::<u64>().ok())
    {
        Some(id) if id > 0 => id,
        _ => return,
    };

    let mut line = format!("🛠️ <@{user_id}> (`{user_name}`) a utilisé **/{full_command}**");
    if let Some(r) = reason.map(str::trim).filter(|r| !r.is_empty()) {
        // Tronque les raisons tres longues pour garder une ligne lisible.
        let r: String = r.chars().take(200).collect();
        line.push_str(&format!("\n> 📝 {r}"));
    }
    let embed = serenity::builder::CreateEmbed::new()
        .description(line)
        .color(0x5865F2)
        .timestamp(serenity::model::Timestamp::now());
    let msg = serenity::builder::CreateMessage::new().embed(embed);
    if let Err(e) = ChannelId::new(channel_id)
        .send_message(&ctx.http, msg)
        .await
    {
        warn!(error = %e, "Echec log commande admin");
    }
}

// ── Event handler free functions ──

/// Called on ready — bootstrap watched users + start Redis consumer.
pub async fn on_ready(ctx: &Context) {
    // Garde run-once : `ready` refire a chaque reconnexion gateway -> sans elle,
    // N consumers Redis + N bootstraps s'accumulent (traitement duplique, fuite
    // de taches).
    use std::sync::atomic::{AtomicBool, Ordering};
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let ctx_bootstrap = ctx.clone();
    tokio::spawn(async move {
        watched_users::bootstrap_watched_users(&ctx_bootstrap).await;

        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "audit-bot-watched-cache".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx_bootstrap.clone();
                async move {
                    watched_users::handle_watched_refresh_event(&ctx, &payload_json).await;
                }
            },
        )
        .await;
    });
}

/// Intercepte tous les messages pour les cacher + tracker watched users.
/// Met en cache un message pour pouvoir retrouver son contenu/auteur a la
/// suppression lorsque le module Audit est actif. Les messages de bots sont
/// inclus afin d'identifier — et exclure — leurs editions/suppressions.
pub async fn cache_message(ctx: &Context, msg: &Message) {
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return,
    };

    // Respecte strictement le toggle du dashboard : module inactif signifie
    // qu'aucun nouveau contenu de message n'entre dans le cache d'audit.
    if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
        return;
    }

    let data = ctx.data.read().await;
    if let Some(cache) = data.get::<MessageCacheKey>() {
        cache.store(
            guild_id,
            msg.id,
            message_cache::CachedMessage {
                author_id: msg.author.id.to_string(),
                author_name: msg.author.name.clone(),
                content: msg.content.clone(),
                channel_id: msg.channel_id.to_string(),
                is_bot: msg.author.bot,
            },
        );
    }
}

pub async fn on_message(ctx: &Context, msg: &Message) {
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return,
    };

    if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
        return;
    }

    // Surveillance : tracker les messages des utilisateurs surveilles
    let data = ctx.data.read().await;
    let user_id = msg.author.id.to_string();
    if watched_users::is_watched(&data, &user_id) {
        drop(data);
        let channel_name = msg
            .channel_id
            .to_channel(&ctx.http)
            .await
            .ok()
            .and_then(|c| c.guild())
            .map(|c| c.name.clone());

        watched_users::track_activity(
            ctx,
            &guild_id.to_string(),
            &user_id,
            "message_sent",
            Some(&msg.channel_id.to_string()),
            channel_name.as_deref(),
            Some(&msg.content),
            serde_json::json!({"message_id": msg.id.to_string()}),
        )
        .await;
    }
}

pub async fn on_message_delete(
    ctx: &Context,
    channel_id: ChannelId,
    message_id: MessageId,
    guild_id: Option<GuildId>,
) {
    if let Some(gid) = guild_id {
        if !is_module_enabled(ctx, &gid.to_string(), MODULE_BOT_NAME).await {
            return;
        }
    }
    handlers::message::handle_delete(ctx, channel_id, message_id, guild_id).await;
}

pub async fn on_message_update(
    ctx: &Context,
    old: Option<Message>,
    new: Option<Message>,
    event: MessageUpdateEvent,
) {
    let Some(guild_id) = event.guild_id else {
        return;
    };
    if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
        return;
    }
    handlers::message::handle_update(ctx, old, new, event).await;
}

pub async fn on_message_delete_bulk(
    ctx: &Context,
    channel_id: ChannelId,
    multiple_deleted: Vec<MessageId>,
    guild_id: Option<GuildId>,
) {
    if let Some(gid) = guild_id {
        if !is_module_enabled(ctx, &gid.to_string(), MODULE_BOT_NAME).await {
            return;
        }
    }
    handlers::message::handle_delete_bulk(ctx, channel_id, multiple_deleted, guild_id).await;
}

pub async fn on_member_add(ctx: &Context, new_member: &Member) {
    if !is_module_enabled(ctx, &new_member.guild_id.to_string(), MODULE_BOT_NAME).await {
        return;
    }
    handlers::member::handle_addition(ctx, new_member).await;
}

pub async fn on_member_remove(ctx: &Context, guild_id: GuildId, user: &User) {
    if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
        return;
    }
    handlers::member::handle_removal(ctx, guild_id, user).await;
}

pub async fn on_ban_add(ctx: &Context, guild_id: GuildId, banned_user: &User) {
    if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
        return;
    }
    handlers::member::handle_ban_addition(ctx, guild_id, banned_user).await;
}

pub async fn on_ban_remove(ctx: &Context, guild_id: GuildId, unbanned_user: &User) {
    if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
        return;
    }
    handlers::member::handle_ban_removal(ctx, guild_id, unbanned_user).await;
}

pub async fn on_member_update(
    ctx: &Context,
    old: Option<Member>,
    new: Option<Member>,
    _event: serenity::model::event::GuildMemberUpdateEvent,
) {
    if let Some(ref new_member) = new {
        handlers::member::handle_update(ctx, old, new_member).await;
    }
}

pub async fn on_voice_state_update(ctx: &Context, old: Option<VoiceState>, new: &VoiceState) {
    if let Some(guild_id) = new.guild_id {
        if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
            return;
        }
    }
    handlers::voice::handle_state_update(ctx, old, new).await;
}

pub async fn on_channel_create(ctx: &Context, channel: &serenity::model::channel::GuildChannel) {
    handlers::channel::handle_create(ctx, channel).await;
}

pub async fn on_channel_delete(
    ctx: &Context,
    channel: &serenity::model::channel::GuildChannel,
    messages: Option<Vec<Message>>,
) {
    handlers::channel::handle_delete(ctx, channel, messages).await;
}

pub async fn on_role_create(ctx: &Context, new: &Role) {
    handlers::role::handle_create(ctx, new).await;
}

pub async fn on_role_delete(
    ctx: &Context,
    guild_id: GuildId,
    removed_role_id: RoleId,
    removed_role: Option<Role>,
) {
    handlers::role::handle_delete(ctx, guild_id, removed_role_id, removed_role).await;
}

pub async fn on_role_update(ctx: &Context, old: Option<Role>, new: &Role) {
    handlers::role::handle_update(ctx, old, new).await;
}

pub async fn on_guild_update(
    ctx: &Context,
    old: Option<Guild>,
    new_incomplete: &serenity::model::guild::PartialGuild,
) {
    handlers::guild::handle_update(ctx, old, new_incomplete).await;
}

pub async fn on_thread_create(ctx: &Context, thread: &serenity::model::channel::GuildChannel) {
    handlers::thread::handle_create(ctx, thread).await;
}

pub async fn on_thread_delete(
    ctx: &Context,
    thread: &serenity::model::channel::PartialGuildChannel,
    full_thread: Option<serenity::model::channel::GuildChannel>,
) {
    handlers::thread::handle_delete(ctx, thread, full_thread).await;
}

pub async fn on_invite_create(ctx: &Context, data: &serenity::model::event::InviteCreateEvent) {
    handlers::invite::handle_create(ctx, data).await;
}

pub async fn on_invite_delete(ctx: &Context, data: &serenity::model::event::InviteDeleteEvent) {
    handlers::invite::handle_delete(ctx, data).await;
}
