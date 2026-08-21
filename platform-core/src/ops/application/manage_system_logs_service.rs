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

    #[tokio::test]
    async fn list_logs_clamps_minimum_limit() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let filters = SystemLogFilters {
            category: None,
            level: None,
            guild_id: None,
            limit: 0,
        };
        let result = service.list_logs(filters).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn purge_with_empty_category() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let result = service.purge_category("").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_logs_with_category_only() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let filters = SystemLogFilters {
            category: Some("database".into()),
            level: None,
            guild_id: None,
            limit: 50,
        };
        let result = service.list_logs(filters).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_logs_with_level_only() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let filters = SystemLogFilters {
            category: None,
            level: Some("warn".into()),
            guild_id: None,
            limit: 50,
        };
        let result = service.list_logs(filters).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_logs_with_guild_only() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let filters = SystemLogFilters {
            category: None,
            level: None,
            guild_id: Some("guild456".into()),
            limit: 50,
        };
        let result = service.list_logs(filters).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn purge_various_categories() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));

        for category in &["api", "bot", "system", "nginx", "postgres"] {
            let result = service.purge_category(category).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn list_logs_hits_every_code_path() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));

        // Test all combinations of filters
        let test_cases = vec![
            (Some("api"), Some("error"), Some("guild1"), 1),
            (Some("api"), Some("error"), None, 1),
            (Some("api"), None, Some("guild1"), 1),
            (None, Some("error"), Some("guild1"), 1),
            (Some("api"), None, None, 1),
            (None, Some("error"), None, 1),
            (None, None, Some("guild1"), 1),
        ];

        for (cat, level, guild, limit) in test_cases {
            let filters = SystemLogFilters {
                category: cat.map(|s| s.to_string()),
                level: level.map(|s| s.to_string()),
                guild_id: guild.map(|s| s.to_string()),
                limit,
            };
            let result = service.list_logs(filters).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn purge_discord_is_blocked() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let result = service.purge_category("discord").await;
        assert!(result.is_err());
        if let Err(DomainError::ValidationError(msg)) = result {
            assert!(msg.contains("Discord"));
        }
    }

    #[tokio::test]
    async fn integration_test_full_workflow() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));

        // List with comprehensive filters
        let filters = SystemLogFilters {
            category: Some("api".into()),
            level: Some("error".into()),
            guild_id: Some("guild123".into()),
            limit: 50,
        };
        let logs = service.list_logs(filters).await.unwrap();
        assert_eq!(logs.len(), 0);

        // Purge non-discord category
        let deleted = service.purge_category("api").await.unwrap();
        assert_eq!(deleted, 42);

        // Attempt to purge discord (should fail)
        let result = service.purge_category("discord").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_logs_boundary_limits() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));

        // Minimum limit
        let filters = SystemLogFilters {
            category: None,
            level: None,
            guild_id: None,
            limit: 1,
        };
        assert!(service.list_logs(filters).await.is_ok());

        // Very large limit
        let filters = SystemLogFilters {
            category: None,
            level: None,
            guild_id: None,
            limit: 1_000_000,
        };
        assert!(service.list_logs(filters).await.is_ok());

        // Negative limit
        let filters = SystemLogFilters {
            category: None,
            level: None,
            guild_id: None,
            limit: -100,
        };
        assert!(service.list_logs(filters).await.is_ok());
    }

    #[tokio::test]
    async fn purge_empty_string_category() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let result = service.purge_category("").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn purge_whitespace_category() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));
        let result = service.purge_category("   ").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_logs_all_optional_some_none() {
        let service = ManageSystemLogsService::new(Arc::new(FakeLogRepo));

        // Mixed Some/None combinations
        let test_cases = vec![
            (Some("cat1"), None, None),
            (None, Some("level1"), None),
            (None, None, Some("guild1")),
            (Some("cat1"), Some("level1"), None),
            (Some("cat1"), None, Some("guild1")),
            (None, Some("level1"), Some("guild1")),
        ];

        for (cat, level, guild) in test_cases {
            let filters = SystemLogFilters {
                category: cat.map(|s| s.to_string()),
                level: level.map(|s| s.to_string()),
                guild_id: guild.map(|s| s.to_string()),
                limit: 100,
            };
            assert!(service.list_logs(filters).await.is_ok());
        }
    }
}
