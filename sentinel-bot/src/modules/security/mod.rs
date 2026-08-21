//! Module security — anti-raid, verification utilisateurs, protection serveur.
//! Migre depuis security-bot.

pub const MODULE_BOT_NAME: &str = "security-bot";

pub mod api_client;
mod background;
mod captcha_handler;
mod commands;
pub mod detectors;
mod join_handler;
pub mod lockdown_expired_consumer;
pub mod porte;
pub mod quarantine_expired_consumer;
pub mod quarantine_reminder_consumer;
pub mod raid_suggest_handler;
pub mod slowmode_expired_consumer;

use chrono::DateTime;
use serenity::all::{CommandInteraction, ComponentInteraction, Context, CreateCommand};
use serenity::model::guild::Member;
use serenity::model::id::GuildId;
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::shared::discord_helpers::{
    is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
};
use crate::shared::heartbeat::ApiClientKey;

use api_client::{ApiClient, MemberPayload, SyncMembersPayload, UpdateMemberPayload};
use detectors::captcha::{self, CaptchaPending};
use detectors::lockdown::LockdownManager;
use detectors::quarantine::QuarantineManager;
use detectors::raid_analyzer::RecentJoinsTracker;
use detectors::raid_detector::RaidDetector;
use detectors::raid_suggest::RaidSuggestGuard;
use detectors::slowmode::SlowmodeManager;

pub use background::spawn_background;

// ── TypeMap keys ──

pub struct SecurityApiKey;
impl TypeMapKey for SecurityApiKey {
    type Value = ApiClient;
}

pub struct RaidDetectorKey;
impl TypeMapKey for RaidDetectorKey {
    type Value = RaidDetector;
}

pub struct SecurityConfigKey;
impl TypeMapKey for SecurityConfigKey {
    type Value = SecurityConfig;
}

pub struct QuarantineKey;
impl TypeMapKey for QuarantineKey {
    type Value = QuarantineManager;
}

pub struct SlowmodeKey;
impl TypeMapKey for SlowmodeKey {
    type Value = SlowmodeManager;
}

pub struct LockdownKey;
impl TypeMapKey for LockdownKey {
    type Value = LockdownManager;
}

pub struct RecentJoinsKey;
impl TypeMapKey for RecentJoinsKey {
    type Value = RecentJoinsTracker;
}

pub struct CaptchaPendingKey;
impl TypeMapKey for CaptchaPendingKey {
    type Value = CaptchaPending;
}

pub struct RaidSuggestGuardKey;
impl TypeMapKey for RaidSuggestGuardKey {
    type Value = RaidSuggestGuard;
}

// ── Security config (loaded from env, stored in TypeMap) ──

use crate::shared::config::{load_env, load_env_bool, load_env_optional, load_env_string};

#[derive(Clone)]
pub struct SecurityConfig {
    pub raid_join_threshold: u64,
    pub raid_join_window_secs: u64,
    pub min_account_age_secs: u64,
    pub quarantine_role_id: Option<u64>,
    pub quarantine_enabled: bool,
    pub slowmode_seconds: u16,
    pub slowmode_duration_secs: u64,
    pub captcha_enabled: bool,
    // `captcha_timeout_secs` a disparu d'ici : le delai avant expulsion est
    // devenu un reglage du SERVEUR (`quarantine_timeout_secs`), lu par l'API.
    // Le garder en variable d'environnement aurait laisse un bouton
    // deconnecte : regle, sans effet, et personne pour le dire.
    pub captcha_type: String,
    pub lockdown_enabled: bool,
    pub lockdown_duration_secs: u64,
    pub alt_detection_enabled: bool,
    pub raid_pattern_enabled: bool,
    pub raid_pattern_score_threshold: u32,
}

impl SecurityConfig {
    pub fn from_env() -> Self {
        Self {
            raid_join_threshold: load_env("RAID_JOIN_THRESHOLD", 10),
            raid_join_window_secs: load_env("RAID_JOIN_WINDOW_SECS", 10),
            min_account_age_secs: load_env("MIN_ACCOUNT_AGE_SECS", 86400),
            quarantine_role_id: load_env_optional("QUARANTINE_ROLE_ID"),
            quarantine_enabled: load_env_bool("QUARANTINE_ENABLED", false),
            slowmode_seconds: load_env("SLOWMODE_SECONDS", 10),
            slowmode_duration_secs: load_env("SLOWMODE_DURATION_SECS", 300),
            captcha_enabled: load_env_bool("CAPTCHA_ENABLED", false),
            captcha_type: {
                let ct = load_env_string("CAPTCHA_TYPE", "button");
                if ct != "button" && ct != "math" {
                    tracing::warn!(value=%ct, "CAPTCHA_TYPE invalide, utilisation de 'button' par defaut");
                    "button".to_string()
                } else {
                    ct
                }
            },
            lockdown_enabled: load_env_bool("LOCKDOWN_ENABLED", false),
            lockdown_duration_secs: load_env("LOCKDOWN_DURATION_SECS", 300),
            alt_detection_enabled: load_env_bool("ALT_DETECTION_ENABLED", false),
            raid_pattern_enabled: load_env_bool("RAID_PATTERN_ENABLED", true),
            raid_pattern_score_threshold: load_env("RAID_PATTERN_SCORE_THRESHOLD", 60),
        }
    }
}

// ── Init TypeMapKeys ──

use std::sync::Arc;

/// Insere tous les TypeMapKeys du module security (trackers + config + API client).
pub fn init_typemap(
    data: &mut serenity::prelude::TypeMap,
    grpc: &Arc<crate::shared::grpc_client::SentinelGrpcClient>,
) {
    let sec_config = SecurityConfig::from_env();
    data.insert::<SecurityApiKey>(ApiClient::new(Arc::clone(grpc)));
    data.insert::<RaidDetectorKey>(RaidDetector::new(
        sec_config.raid_join_threshold,
        sec_config.raid_join_window_secs,
    ));
    data.insert::<QuarantineKey>(QuarantineManager::new());
    data.insert::<SlowmodeKey>(SlowmodeManager::new());
    data.insert::<LockdownKey>(LockdownManager::new());
    data.insert::<RecentJoinsKey>(RecentJoinsTracker::new(sec_config.raid_join_window_secs));
    data.insert::<CaptchaPendingKey>(CaptchaPending::new());
    data.insert::<RaidSuggestGuardKey>(RaidSuggestGuard::new());
    data.insert::<SecurityConfigKey>(sec_config);
}

// ── Slash commands ──

pub fn register_commands() -> Vec<CreateCommand> {
    vec![commands::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
    commands::handle(ctx, command).await;
}

// ── Component interaction routing ──

/// Retourne true si ce custom_id est gere par le module security.
pub fn handles_component(cid: &str) -> bool {
    cid.starts_with(captcha::CAPTCHA_BUTTON_PREFIX)
        || cid.starts_with(captcha::CAPTCHA_MATH_PREFIX)
        || raid_suggest_handler::handles_component(cid)
}

// ── Event handlers (appelees depuis handler.rs) ──

/// Sync tous les membres au demarrage (appelee depuis ready).
pub async fn on_ready_sync(ctx: &Context, guilds: &[serenity::model::guild::UnavailableGuild]) {
    let data = ctx.data.read().await;
    let sec_api = match data.get::<SecurityApiKey>() {
        Some(a) => a,
        None => {
            error!("SecurityApiKey manquant pour sync membres");
            return;
        }
    };

    for guild in guilds {
        let guild_id = guild.id;
        match guild_id.members(&ctx.http, None, None).await {
            Ok(members) => {
                let payloads: Vec<MemberPayload> = members
                    .iter()
                    .map(|m| {
                        let roles: Vec<String> = m.roles.iter().map(|r| r.to_string()).collect();
                        MemberPayload {
                            guild_id: guild_id.to_string(),
                            user_id: m.user.id.to_string(),
                            username: m.user.name.clone(),
                            display_name: m.nick.clone(),
                            avatar: m.user.avatar.as_ref().map(|a| a.to_string()),
                            roles: serde_json::json!(roles),
                            joined_at: m
                                .joined_at
                                .and_then(|t| DateTime::from_timestamp(t.unix_timestamp(), 0)),
                            account_created: Some(DateTime::from_timestamp(
                                m.user.created_at().unix_timestamp(),
                                0,
                            ))
                            .flatten(),
                            is_bot: m.user.bot,
                            last_seen_at: None,
                        }
                    })
                    .collect();

                let count = payloads.len();
                let payload = SyncMembersPayload {
                    guild_id: guild_id.to_string(),
                    members: payloads,
                };

                match sec_api.sync_members(&payload).await {
                    Ok(()) => info!(guild_id = %guild_id, members = count, "Membres synchronises"),
                    Err(e) => {
                        error!(guild_id = %guild_id, error = %e, "Erreur sync membres")
                    }
                }
            }
            Err(e) => {
                error!(guild_id = %guild_id, error = %e, "Impossible de recuperer les membres")
            }
        }
    }
}

/// Declenche a chaque nouveau membre qui rejoint un serveur.
pub async fn on_member_add(ctx: &Context, new_member: &Member) {
    join_handler::on_member_add(ctx, new_member).await
}

/// Declenche quand un membre quitte le serveur.
pub async fn on_member_remove(
    ctx: &Context,
    guild_id: GuildId,
    user: &serenity::model::user::User,
) {
    info!(guild_id = %guild_id, user = %user.name, "Membre parti (security)");

    let data = ctx.data.read().await;

    // NE PAS hard-delete la ligne guild_members ici : le lifecycle
    // `/api/members/{guild}/{user}/leave` (handler.rs::guild_member_removal)
    // fait un soft-delete (left_at = NOW()) qui PRESERVE la ligne. C'est
    // indispensable pour reconnaitre un membre qui revient (is_known_member ->
    // message de RE-bienvenue au lieu de bienvenue). L'ancien remove_member
    // (DELETE) ecrasait ce soft-delete et cassait la detection de retour.
    // Les listes de jeu filtrent deja `left_at` cote requete.

    if let Some(base) = data.get::<ApiClientKey>() {
        base.send_log(
            "info",
            &guild_id.to_string(),
            &format!("Membre parti : {} ({})", user.name, user.id),
        );
    }
}

/// Declenche quand un membre est mis a jour (pseudo, roles, avatar).
pub async fn on_member_update(ctx: &Context, member: &Member) {
    let guild_id = member.guild_id;
    let user = &member.user;

    let data = ctx.data.read().await;
    if let Some(sec_api) = data.get::<SecurityApiKey>() {
        let roles: Vec<String> = member.roles.iter().map(|r| r.to_string()).collect();
        let payload = UpdateMemberPayload {
            username: Some(user.name.clone()),
            display_name: member.nick.clone(),
            avatar: user.avatar.as_ref().map(|a| a.to_string()),
            roles: Some(serde_json::json!(roles)),
        };
        if let Err(e) = sec_api
            .update_member(&guild_id.to_string(), &user.id.to_string(), &payload)
            .await
        {
            warn!(error = %e, "Erreur update_member");
        }
    }
}

/// Declenche quand un membre est banni.
pub async fn on_ban_add(
    ctx: &Context,
    guild_id: GuildId,
    banned_user: &serenity::model::user::User,
) {
    info!(guild_id = %guild_id, user = %banned_user.name, "Membre banni (security)");

    let data = ctx.data.read().await;
    if let Some(base) = data.get::<ApiClientKey>() {
        base.send_log(
            "warn",
            &guild_id.to_string(),
            &format!("Membre banni : {} ({})", banned_user.name, banned_user.id),
        );
    }
}

/// Declenche quand un membre est debanni.
pub async fn on_ban_remove(
    ctx: &Context,
    guild_id: GuildId,
    unbanned_user: &serenity::model::user::User,
) {
    info!(guild_id = %guild_id, user = %unbanned_user.name, "Membre debanni (security)");

    let data = ctx.data.read().await;
    if let Some(base) = data.get::<ApiClientKey>() {
        base.send_log(
            "info",
            &guild_id.to_string(),
            &format!(
                "Membre debanni : {} ({})",
                unbanned_user.name, unbanned_user.id
            ),
        );
    }
}

/// Gere les interactions captcha (bouton + math).
pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }
    if raid_suggest_handler::handles_component(&component.data.custom_id) {
        raid_suggest_handler::on_component(ctx, component).await;
        return;
    }
    captcha_handler::on_component(ctx, component).await
}
