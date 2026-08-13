use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::weekly_report::WeeklyReport;
use crate::sentinel::domain::entities::audit::weekly_report::WEEKLY_REPORT_WINDOW_DAYS;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::audit::get_weekly_report::GetWeeklyReportUseCase;
use crate::sentinel::ports::outbound::audit::audit_event_counter::AuditEventCounter;

/// Service d'agregation du rapport hebdomadaire.
///
/// Compte les events d'audit par type sur la fenetre de 7 jours (via le port
/// outbound) puis mappe vers les compteurs metier. Coeur pur : aucun formatage,
/// aucune dependance infra.
pub struct GetWeeklyReportService {
    counter: Arc<dyn AuditEventCounter>,
}

impl GetWeeklyReportService {
    pub fn new(counter: Arc<dyn AuditEventCounter>) -> Self {
        Self { counter }
    }
}

#[async_trait]
impl GetWeeklyReportUseCase for GetWeeklyReportService {
    async fn get(&self, guild_id: &str) -> Result<WeeklyReport, DomainError> {
        let counts = self
            .counter
            .count_by_event_type(guild_id, WEEKLY_REPORT_WINDOW_DAYS)
            .await?;
        Ok(WeeklyReport::from_event_counts(counts))
    }
}

#[cfg(test)]
#[path = "tests/get_weekly_report.rs"]
mod tests;
