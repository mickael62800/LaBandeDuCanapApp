//! Vues d'etat pour les fonctionnalites qui composent plusieurs domaines.
//!
//! Ce ne sont pas de nouveaux domaines : chaque structure est la liste exacte
//! des ports necessaires a un groupe de handlers transversal. `FromRef` les
//! derive de la composition root sans dupliquer les services.

use std::sync::Arc;

use axum::extract::FromRef;
use ops_core::ports::outbound::log_repository::LogRepository;
use ops_core::ports::outbound::service_registry::ServiceRegistry;
use sentinel_core::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase;
use sentinel_core::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use sentinel_core::ports::inbound::community::manage_voice_channels::ManageVoiceChannelsUseCase;
use sentinel_core::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use sentinel_core::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use sentinel_core::ports::inbound::moderation::manage_rules::ManageRulesUseCase;
use sentinel_core::ports::inbound::system::manage_bot_persistence::ManageBotPersistenceUseCase;
use sentinel_core::ports::inbound::system::manage_tickets::ManageTicketsUseCase;
use sentinel_core::ports::outbound::community::sponsorship_repository::SponsorshipRepository;
use sentinel_core::ports::outbound::community::temp_role_repository::TempRoleRepository;
use sentinel_core::ports::outbound::moderation::pending_action_repository::PendingActionRepository;
use sentinel_core::ports::outbound::system::guild_repository::GuildRepository;

use crate::adapters::outbound::nexus_games::NexusGamesClient;
use crate::adapters::outbound::redis_cache::RedisCache;
use crate::adapters::outbound::ws::broadcaster::EventBroadcaster;

use super::AppState;

#[derive(Clone)]
pub struct DashboardState {
    pub stats_uc: Arc<dyn ManageStatsUseCase>,
    pub service_registry: Arc<dyn ServiceRegistry>,
    pub infractions_uc: Arc<dyn ManageInfractionsUseCase>,
    pub moderation_uc: Arc<dyn ManageModerationUseCase>,
    pub rules_uc: Arc<dyn ManageRulesUseCase>,
    pub guild_repo: Arc<dyn GuildRepository>,
    pub log_repo: Arc<dyn LogRepository>,
    pub broadcaster: Arc<EventBroadcaster>,
    pub redis_client: redis::Client,
}

impl FromRef<AppState> for DashboardState {
    fn from_ref(state: &AppState) -> Self {
        Self {
            stats_uc: state.audit.stats_uc.clone(),
            service_registry: state.ops.service_registry.clone(),
            infractions_uc: state.moderation.infractions_uc.clone(),
            moderation_uc: state.moderation.moderation_uc.clone(),
            rules_uc: state.moderation.rules_uc.clone(),
            guild_repo: state.system.guild_repo.clone(),
            log_repo: state.shared.log_repo.clone(),
            broadcaster: state.shared.broadcaster.clone(),
            redis_client: state.shared.redis_client.clone(),
        }
    }
}

/// Salons vocaux temporaires.
///
/// `tickets_uc` et `superadmin_user_ids` en sont sortis avec le scope par role
/// de `list_all_channels` : le premier n'y servait qu'a appeler
/// `moderated_guilds`, retire parce qu'il lisait une table supprimee par la
/// migration 007. Ce sous-etat ne reclame donc plus de port etranger a son
/// domaine, hors `audit_logs_uc` qui trace ses propres actions.
#[derive(Clone)]
pub struct VoiceChannelsState {
    pub voice_channels_uc: Arc<dyn ManageVoiceChannelsUseCase>,
    pub audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>,
    pub broadcaster: Arc<EventBroadcaster>,
}

impl FromRef<AppState> for VoiceChannelsState {
    fn from_ref(state: &AppState) -> Self {
        Self {
            voice_channels_uc: state.community.voice_channels_uc.clone(),
            audit_logs_uc: state.audit.audit_logs_uc.clone(),
            broadcaster: state.shared.broadcaster.clone(),
        }
    }
}

#[derive(Clone)]
pub struct BotPersistenceState {
    pub audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>,
    pub bot_persistence_uc: Arc<dyn ManageBotPersistenceUseCase>,
    pub tickets_uc: Arc<dyn ManageTicketsUseCase>,
    pub sponsorship_repo: Arc<dyn SponsorshipRepository>,
    pub temp_role_repo: Arc<dyn TempRoleRepository>,
    pub pending_action_repo: Arc<dyn PendingActionRepository>,
}

impl FromRef<AppState> for BotPersistenceState {
    fn from_ref(state: &AppState) -> Self {
        Self {
            audit_logs_uc: state.audit.audit_logs_uc.clone(),
            bot_persistence_uc: state.system.bot_persistence_uc.clone(),
            tickets_uc: state.system.tickets_uc.clone(),
            sponsorship_repo: state.community.sponsorship_repo.clone(),
            temp_role_repo: state.community.temp_role_repo.clone(),
            pending_action_repo: state.moderation.pending_action_repo.clone(),
        }
    }
}

#[derive(Clone)]
pub struct PurgeState {
    pub infractions_uc: Arc<dyn ManageInfractionsUseCase>,
    pub audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>,
    pub log_repo: Arc<dyn LogRepository>,
    pub broadcaster: Arc<EventBroadcaster>,
}

impl FromRef<AppState> for PurgeState {
    fn from_ref(state: &AppState) -> Self {
        Self {
            infractions_uc: state.moderation.infractions_uc.clone(),
            audit_logs_uc: state.audit.audit_logs_uc.clone(),
            log_repo: state.shared.log_repo.clone(),
            broadcaster: state.shared.broadcaster.clone(),
        }
    }
}

#[derive(Clone)]
pub struct NexusGamesState {
    pub guild_id: String,
    pub nexus_games: Arc<NexusGamesClient>,
}

impl FromRef<AppState> for NexusGamesState {
    fn from_ref(state: &AppState) -> Self {
        Self {
            guild_id: state.shared.guild_id.clone(),
            nexus_games: state.shared.nexus_games.clone(),
        }
    }
}

#[derive(Clone)]
pub struct CacheStatsState {
    pub cache: Option<Arc<RedisCache>>,
}

impl FromRef<AppState> for CacheStatsState {
    fn from_ref(state: &AppState) -> Self {
        Self {
            cache: state.shared.cache.clone(),
        }
    }
}
