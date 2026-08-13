//! Etat du domaine audit : journal Discord, surveillance, statistiques,
//! detection d'anomalies et evenements de securite.

use std::sync::Arc;

use axum::extract::FromRef;
use platform_core::sentinel::ports::inbound::audit::detect_moderation_anomaly::DetectModerationAnomalyUseCase;
use platform_core::sentinel::ports::inbound::audit::get_weekly_report::GetWeeklyReportUseCase;
use platform_core::sentinel::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase;
use platform_core::sentinel::ports::inbound::audit::manage_discord_action_messages::ManageDiscordActionMessagesUseCase;
use platform_core::sentinel::ports::inbound::audit::manage_security::ManageSecurityUseCase;
use platform_core::sentinel::ports::inbound::audit::manage_snapshots::ManageSnapshotsUseCase;
use platform_core::sentinel::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use platform_core::sentinel::ports::inbound::audit::manage_watched_users::ManageWatchedUsersUseCase;
use platform_core::sentinel::ports::outbound::audit::analytics_repository::AnalyticsRepository;
use platform_core::sentinel::ports::outbound::audit::user_activity_repository::UserActivityRepository;
use platform_core::sentinel::ports::outbound::community::daily_activity_repository::DailyActivityRepository;
use platform_core::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;

use crate::sentinel::adapters::outbound::discord_api::DiscordApi;
use crate::sentinel::adapters::outbound::ws::broadcaster::EventBroadcaster;
use crate::sentinel::bootstrap::state::AppState;

/// Ports de l'audit et de l'observation du serveur.
///
/// Domaine en lecture pour l'essentiel : il agrege ce que les autres ont
/// produit (audit-logs Discord ingeres par le worker, activite, infractions)
/// pour alimenter le dashboard et les rapports.
#[derive(Clone)]
pub struct AuditState {
    pub audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>,
    pub watched_users_uc: Arc<dyn ManageWatchedUsersUseCase>,
    pub stats_uc: Arc<dyn ManageStatsUseCase>,
    pub detect_anomaly_uc: Arc<dyn DetectModerationAnomalyUseCase>,
    pub weekly_report_uc: Arc<dyn GetWeeklyReportUseCase>,
    pub snapshots_uc: Arc<dyn ManageSnapshotsUseCase>,
    pub discord_action_messages_uc: Arc<dyn ManageDiscordActionMessagesUseCase>,
    /// Detection raid / comptes alt. Rattache a l'audit et non a la
    /// moderation : il produit des *observations* (`security_events`), pas
    /// des sanctions.
    pub security_uc: Arc<dyn ManageSecurityUseCase>,
    pub analytics_repo: Arc<dyn AnalyticsRepository>,
    pub user_activity_repo: Arc<dyn UserActivityRepository>,

    // ── Dependances transverses du domaine ──
    pub broadcaster: Arc<EventBroadcaster>,
    /// Reglages par serveur (intervalles de publication, seuils de rapport).
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    /// Cache des agregats analytics (heatmaps, trends), couteux a recalculer.
    /// Utilise directement par `handlers/audit/analytics.rs`.
    pub redis_client: redis::Client,
    /// Port community consomme par les snapshots d'activite : le classement
    /// des membres actifs vit cote community, l'instantane cote audit.
    pub daily_activity_repo: Arc<dyn DailyActivityRepository>,
    /// Publication des snapshots (top membres) dans un salon Discord.
    ///
    /// Remplace un `reqwest::Client` construit dans le handler avec le token
    /// brut : un adaptateur inbound qui fait de l'I/O sortante court-circuite
    /// le port et rend le handler intestable sans reseau.
    pub discord_api: Arc<dyn DiscordApi>,
}

impl FromRef<AppState> for AuditState {
    fn from_ref(state: &AppState) -> Self {
        state.audit.clone()
    }
}
