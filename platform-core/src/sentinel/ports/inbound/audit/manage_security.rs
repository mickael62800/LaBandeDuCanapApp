use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::security_event::SecurityEvent;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::domain::services::audit::security_analyzer::JoinInfo;

pub struct ReportSecurityEventCommand {
    pub guild_id: GuildId,
    pub event_type: String,
    pub severity: String,
    pub description: String,
    pub user_ids: Vec<String>,
}

pub struct AnalyzeNewMemberCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub has_avatar: bool,
    pub account_created_timestamp: i64,
    pub is_bot: bool,
    pub recent_joins: Vec<JoinInfo>,
    /// Flood de vitesse detecte cote bot (buffer local `RaidDetector`).
    /// Force une reponse guild-wide en mode auto/hybrid (raid massif).
    pub is_velocity_raid: bool,
}

/// Decision de securite retournee par l'API apres analyse.
#[derive(Debug, Clone, Default)]
pub struct SecurityDecision {
    pub is_raid: bool,
    pub raid_score: u32,
    pub is_suspicious_account: bool,
    pub is_alt_account: bool,
    pub alt_similar_to: String,
    pub quarantine: bool,
    pub send_captcha: bool,
    pub activate_lockdown: bool,
    pub slowmode_secs: u32,
    /// `true` quand la reponse GUILD-WIDE (lockdown / slowmode / bump
    /// verification) doit etre SUGGEREE au staff plutot qu'appliquee
    /// automatiquement (mode `suggest` ou `hybrid` sous le seuil auto).
    /// N'a de sens que si une action guild-wide est presente ; la
    /// quarantaine + le captcha restent toujours appliques par le bot.
    pub suggest_only: bool,
    pub event_type: String,
    pub event_description: String,
}

#[async_trait]
pub trait ManageSecurityUseCase: Send + Sync {
    async fn report_event(
        &self,
        command: ReportSecurityEventCommand,
    ) -> Result<SecurityEvent, DomainError>;
    async fn list_events(&self, guild_id: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError>;
    /// Purge les evenements de securite d'une guilde (+ auto-watches). Renvoie
    /// (nb_events_supprimes, nb_watches_supprimes).
    async fn purge_events(&self, guild_id: &str) -> Result<(u64, u64), DomainError>;

    /// Analyse un nouveau membre : raid, compte suspect, alt detection.
    /// L'API decide de tout et retourne les actions a executer par le bot.
    async fn analyze_new_member(
        &self,
        command: AnalyzeNewMemberCommand,
    ) -> Result<SecurityDecision, DomainError>;
}
