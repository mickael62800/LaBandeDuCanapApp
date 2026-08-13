//! Use case lecture des statistiques de moderation (breakdown par moderateur
//! + tendance quotidienne). Read-only, agrege depuis `audit_logs`.

use async_trait::async_trait;

use crate::sentinel::domain::entities::moderation::modstats::ModeratorBreakdown;
use crate::sentinel::domain::entities::moderation::modstats::ModstatsTrendDay;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ReadModstatsUseCase: Send + Sync {
    /// Breakdown par moderateur sur `days` jours (clampe 1..=90), top 20.
    async fn modstats(
        &self,
        guild_id: &str,
        days: i32,
    ) -> Result<Vec<ModeratorBreakdown>, DomainError>;
    /// Tendance quotidienne sur `days` jours (clampe 1..=90).
    async fn modstats_trend(
        &self,
        guild_id: &str,
        days: i32,
    ) -> Result<Vec<ModstatsTrendDay>, DomainError>;
}
