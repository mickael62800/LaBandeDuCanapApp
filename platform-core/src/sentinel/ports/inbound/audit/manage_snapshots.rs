use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::snapshot::{JobReport, TopPublishPlan};
use crate::sentinel::domain::errors::DomainError;

/// Use case des jobs analytics declenches par le worker. Toute la regle metier
/// (lecture de config par guild, flags, deltas de baseline, filtres de
/// publication) vit ici ; les handlers HTTP restent minces.
#[async_trait]
pub trait ManageSnapshotsUseCase: Send + Sync {
    /// Snapshot quotidien de toutes les guilds actives (baseline figee).
    async fn snapshot_daily_all(&self) -> Result<JobReport, DomainError>;

    /// Snapshot horaire de toutes les guilds actives.
    async fn snapshot_hourly_all(&self) -> Result<JobReport, DomainError>;

    /// Purge des donnees analytics au dela des retentions configurees.
    async fn retention_cleanup_all(&self) -> Result<JobReport, DomainError>;

    /// Calcule les publications "Top users" dues (le POST Discord reste au
    /// handler). Renvoie aussi le nombre de guilds skip.
    async fn plan_top_publications(&self) -> Result<TopPublishPlan, DomainError>;

    /// Persiste l'horodatage du dernier post reussi pour une guild.
    async fn mark_top_published(
        &self,
        guild_id: &str,
        published_at: &str,
    ) -> Result<(), DomainError>;
}
