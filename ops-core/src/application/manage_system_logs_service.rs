//! Use case lecture/purge des logs systeme. Le SQL vit dans `LogRepository`.

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::log_entry::LogEntry;
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_system_logs::{
    ManageSystemLogsUseCase, SystemLogFilters,
};
use crate::ports::outbound::log_repository::LogRepository;

pub struct ManageSystemLogsService {
    repo: Arc<dyn LogRepository>,
}

impl ManageSystemLogsService {
    pub fn new(repo: Arc<dyn LogRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageSystemLogsUseCase for ManageSystemLogsService {
    async fn list_logs(&self, filters: SystemLogFilters) -> Result<Vec<LogEntry>, DomainError> {
        let limit = filters
            .limit
            .clamp(1, crate::application::BATCH_LIMIT_MAX);
        self.repo
            .find_filtered(
                filters.category.as_deref(),
                filters.level.as_deref(),
                filters.guild_id.as_deref(),
                limit,
            )
            .await
    }

    async fn purge_category(&self, category: &str) -> Result<u64, DomainError> {
        if category == "discord" {
            return Err(DomainError::ValidationError(
                "Impossible de supprimer les journaux Discord".into(),
            ));
        }
        self.repo.delete_by_category(category).await
    }
}
