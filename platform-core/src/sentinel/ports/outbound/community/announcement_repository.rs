use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::sentinel::domain::entities::community::announcement::{
    AnnouncementRun, ButtonInteraction, ChannelPostResult, RunStatus, ScheduledAnnouncement,
};
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait AnnouncementRepository: Send + Sync {
    async fn create(&self, ann: &ScheduledAnnouncement) -> Result<(), DomainError>;
    async fn update(&self, ann: &ScheduledAnnouncement) -> Result<(), DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<ScheduledAnnouncement>, DomainError>;
    async fn list_by_guild(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ScheduledAnnouncement>, DomainError>;

    /// Toggle enabled. Retourne le nouvel etat.
    async fn set_enabled(&self, id: Uuid, enabled: bool) -> Result<bool, DomainError>;

    /// Vu par le worker : annonces dont next_run_at <= now ET enabled=TRUE.
    /// Limite a `limit` pour eviter de bloquer le worker sur un gros backlog.
    async fn list_due(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<ScheduledAnnouncement>, DomainError>;

    /// Met a jour next_run_at + last_run_at. Si next_run_at est None (annonce
    /// terminee : Once deja tourne ou end_date depasse), passe enabled = FALSE
    /// pour ne plus etre selectionne.
    async fn mark_run(
        &self,
        id: Uuid,
        last_run_at: DateTime<Utc>,
        next_run_at: Option<DateTime<Utc>>,
    ) -> Result<(), DomainError>;

    // ── Historique ──────────────────────────────────────────────────────

    async fn insert_run(&self, run: &AnnouncementRun) -> Result<(), DomainError>;

    async fn update_run_result(
        &self,
        run_id: Uuid,
        status: RunStatus,
        channels_posted: &[ChannelPostResult],
        error: Option<&str>,
    ) -> Result<(), DomainError>;

    async fn list_runs(
        &self,
        announcement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AnnouncementRun>, DomainError>;

    /// Insert une interaction sur un bouton (cliqué par un user).
    async fn record_button_interaction(
        &self,
        interaction: &ButtonInteraction,
    ) -> Result<(), DomainError>;

    /// Liste des interactions sur les boutons d'une annonce.
    async fn list_button_interactions(
        &self,
        announcement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ButtonInteraction>, DomainError>;

    // ── Retention ───────────────────────────────────────────────────────

    /// Liste les identifiants de toutes les guilds (pour les jobs globaux).
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError>;

    /// Supprime les runs d'annonces plus vieux que `days` jours pour une
    /// guild. Retourne le nombre de lignes supprimees.
    async fn delete_runs_older_than(&self, guild_id: &str, days: i32) -> Result<u64, DomainError>;
}
