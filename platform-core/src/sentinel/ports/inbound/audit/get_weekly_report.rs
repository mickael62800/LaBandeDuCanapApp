use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::weekly_report::WeeklyReport;
use crate::sentinel::domain::errors::DomainError;

/// Use case : agrege server-side le rapport d'activite hebdomadaire d'un guild
/// depuis les events d'audit deja persistes. Remplace l'ancien `WeeklyTracker`
/// du bot (agregation RAM). Le formatage embed reste cote bot.
#[async_trait]
pub trait GetWeeklyReportUseCase: Send + Sync {
    async fn get(&self, guild_id: &str) -> Result<WeeklyReport, DomainError>;
}
