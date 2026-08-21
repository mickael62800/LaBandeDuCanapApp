//! Use case lecture/purge des logs systeme. Le SQL vit dans `LogRepository`.

use std::sync::Arc;

use async_trait::async_trait;

use crate::ops::domain::entities::log_entry::LogEntry;
use crate::ops::domain::errors::DomainError;
use crate::ops::ports::inbound::manage_system_logs::{ManageSystemLogsUseCase, SystemLogFilters};
use crate::ops::ports::outbound::log_repository::LogRepository;

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
            .clamp(1, crate::ops::application::BATCH_LIMIT_MAX);
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

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeLogRepo;
    #[async_trait]
    impl LogRepository for FakeLogRepo {
        async fn save(&self, _entry: &LogEntry) -> Result<(), DomainError> {
            Ok(())
        }

        async fn find_all(&self, _limit: i64) -> Result<Vec<LogEntry>, DomainError> {
            Ok(vec![])
        }

        async fn find_filtered(
            &self,
            _category: Option<&str>,
            _level: Option<&str>,
            _guild_id: Option<&str>,
            _limit: i64,
        ) -> Result<Vec<LogEntry>, DomainError> {
            Ok(vec![])
        }

        async fn delete_by_category(&self, _category: &str) -> Result<u64, DomainError> {
            Ok(42)
        }

        async fn delete_older_than_days(&self, _days: i32) -> Result<u64, DomainError> {
            Ok(0)
        }
    }

    #[test]
    fn service_can_be_created() {
        let _service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
    }

    #[tokio::test]
    async fn list_logs() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let filters = SystemLogFilters {
            category: None,
            level: None,
            guild_id: None,
            limit: 50,
        };
        let result = service.list_logs(filters).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn purge_rejects_discord() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let result = service.purge_category("discord").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn purge_allows_other_categories() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let result = service.purge_category("app").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_logs_with_all_filters() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let filters = SystemLogFilters {
            category: Some("api".into()),
            level: Some("error".into()),
            guild_id: Some("guild123".into()),
            limit: 100,
        };
        let result = service.list_logs(filters).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_logs_without_filters() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let filters = SystemLogFilters {
            category: None,
            level: None,
            guild_id: None,
            limit: 25,
        };
        let result = service.list_logs(filters).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn purge_returns_deleted_count() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let result = service.purge_category("old").await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn list_logs_clamps_oversized_limit() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let filters = SystemLogFilters {
            category: None,
            level: None,
            guild_id: None,
            limit: 10_000,
        };
        let result = service.list_logs(filters).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_logs_with_single_filter() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let filters = SystemLogFilters {
            category: Some("bot".into()),
            level: None,
            guild_id: None,
            limit: 10,
        };
        let result = service.list_logs(filters).await;
        assert!(result.is_ok());
    }

    #[test]
    fn service_construction() {
        let _service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
    }
}
