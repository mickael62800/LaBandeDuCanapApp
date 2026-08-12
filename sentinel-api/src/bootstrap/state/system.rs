//! Etat du domaine system : le METIER de la plateforme Sentinel.
//!
//! Tickets, OAuth, reset de guilde, lockdown, slowmode, quarantaine, exports.
//! Tout cela parle de Discord et du service rendu au serveur.
//!
//! Ce qui n'est PLUS ici : l'exploitation de la MACHINE (Docker, sondes,
//! logs techniques, securite de l'hote, regles d'alerte). Ce domaine etait
//! melange a celui-ci alors qu'il concerne autant Nexus et Atrium ; il vit
//! desormais dans `OpsState` (cf. `bootstrap/state/ops.rs`).
//!
//! Les scalaires et clients transverses lus par les middlewares vivent dans
//! `SharedState`. La liste des superadmins reste ici car elle est aussi une
//! donnee fonctionnelle exposee par certains handlers systeme.

use std::sync::Arc;

use axum::extract::FromRef;
use sentinel_core::ports::inbound::system::manage_bot_persistence::ManageBotPersistenceUseCase;
use sentinel_core::ports::inbound::system::manage_export_jobs::ManageExportJobsUseCase;
use sentinel_core::ports::inbound::system::manage_lockdown::ManageLockdownUseCase;
use sentinel_core::ports::inbound::system::manage_quarantine::ManageQuarantineUseCase;
use sentinel_core::ports::inbound::system::manage_slowmode::ManageSlowmodeUseCase;
use sentinel_core::ports::inbound::system::manage_tickets::ManageTicketsUseCase;
use sentinel_core::ports::inbound::system::reset_guild::ResetGuildUseCase;
use sentinel_core::ports::outbound::system::bot_config_repository::BotConfigRepository;
use sentinel_core::ports::outbound::system::guild_repository::GuildRepository;

use crate::adapters::outbound::discord_api::DiscordApi;
use crate::adapters::outbound::ws::broadcaster::EventBroadcaster;
use crate::bootstrap::state::AppState;

/// Ports du metier de la plateforme Sentinel.
#[derive(Clone)]
pub struct SystemState {
    // ── Support et vie du service ──
    pub tickets_uc: Arc<dyn ManageTicketsUseCase>,
    pub reset_guild_uc: Arc<dyn ResetGuildUseCase>,
    pub bot_persistence_uc: Arc<dyn ManageBotPersistenceUseCase>,

    // ── Mesures de crise sur le serveur Discord ──
    pub quarantine_uc: Arc<dyn ManageQuarantineUseCase>,
    pub lockdown_uc: Arc<dyn ManageLockdownUseCase>,
    pub slowmode_uc: Arc<dyn ManageSlowmodeUseCase>,

    // ── Exports ──
    pub export_uc:
        Arc<dyn sentinel_core::application::system::export_service::ExecuteExportUseCase>,
    pub export_jobs_uc: Arc<dyn ManageExportJobsUseCase>,
    pub guild_repo: Arc<dyn GuildRepository>,

    // ── Dependances transverses du domaine ──
    pub broadcaster: Arc<EventBroadcaster>,
    pub discord_api: Arc<dyn DiscordApi>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    pub redis_client: redis::Client,

    // La configuration OAuth (`DISCORD_CLIENT_ID/SECRET/REDIRECT_URI`,
    // `WEB_FRONT_URL`) a suivi le flux dans `auth-api`. Elle n'a plus de
    // lecteur ici.
    // `superadmin_user_ids` a ete retire : c'est `auth-api` qui tranche, et les
    // deux derniers lecteurs (`list_tickets`, `list_all_channels`) comparaient
    // l'identite a cette liste LOCALE avant de retomber sur `moderated_guilds`,
    // lui-meme casse depuis la migration 007. Une liste locale qui double la
    // decision de l'identite est exactement la divergence que l'extraction de
    // `auth-api` a supprimee.
    /// Secret HMAC partage bot <-> API, PAS un jeton d'authentification ici.
    /// `guild_reset` signe son event Redis avec : sans cette signature, publier
    /// sur la stream suffirait a declencher un reset destructif (unban-all +
    /// strip-roles) sur un serveur. Cf. `handlers/system/guild_reset.rs`.
    pub api_key: String,
}

impl FromRef<AppState> for SystemState {
    fn from_ref(state: &AppState) -> Self {
        state.system.clone()
    }
}
