//! Logique des 3 jobs du game-portal-worker, exposees via l'API.
//!
//! Ces fonctions sont appelees par les endpoints internes /api/games/internal/jobs/*
//! que le worker invoque sur un timer. Elles utilisent les use cases existants
//! et les ports outbound pour ne pas dupliquer la logique.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::nexus::application::game::config_loader::{
    load_game_portal_config, load_game_portal_configs,
};
use crate::nexus::domain::entities::game::audit::GameAuditAction;
use crate::nexus::domain::entities::game::server::{should_auto_restart, GameServerStatus};
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::outbound::game::container_runtime::{ContainerRuntime, ContainerState};
use crate::nexus::ports::outbound::game::game_audit_repository::GameAuditRepository;
use crate::nexus::ports::outbound::game::game_server_repository::{
    GameServerRepository, GameServerRuntimeUpdate,
};
use crate::nexus::ports::outbound::game::player_session_repository::PlayerSessionRepository;
use crate::nexus::ports::outbound::game::port_allocator::{PortAllocator, PortKind};
use crate::nexus::ports::outbound::game::rcon_client::{RconClient, RconConnectionParams};
use crate::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository;

fn managed_server_id_label(labels: &HashMap<String, String>) -> Option<&str> {
    labels
        .get("nexus.server_id")
        .or_else(|| labels.get("sentinel.server_id"))
        .map(String::as_str)
}

/// Bag d'adapters pour les jobs (evite des signatures kilometriques).
pub struct JobContext {
    pub server_repo: Arc<dyn GameServerRepository>,
    pub template_repo: Arc<
        dyn crate::nexus::ports::outbound::game::game_template_repository::GameTemplateRepository,
    >,
    pub audit_repo: Arc<dyn GameAuditRepository>,
    pub session_repo: Arc<dyn PlayerSessionRepository>,
    pub container_runtime: Arc<dyn ContainerRuntime>,
    pub rcon_client: Arc<dyn RconClient>,
    pub port_allocator: Arc<dyn PortAllocator>,
    pub bot_config: Arc<dyn BotConfigRepository>,
    pub events: Arc<dyn crate::nexus::ports::outbound::events::EventPublisher>,
}

/// Stats retournees par chaque job (pour observabilite worker -> log API).
#[derive(Debug, serde::Serialize)]
pub struct JobReport {
    pub job: &'static str,
    pub processed: usize,
    pub errors: usize,
    pub details: serde_json::Value,
}

mod daily_ping;
mod health_check;
mod idle_shutdown;
mod image_cleanup;
mod reconciler;
mod reveal_ip;

pub use daily_ping::run_daily_ping;
pub use health_check::run_health_check;
pub use idle_shutdown::run_idle_shutdown;
pub use image_cleanup::run_image_cleanup;
pub use reconciler::run_reconciler;
pub use reveal_ip::run_reveal_ip;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nexus::domain::entities::game::audit::{GameAuditAction, GameAuditEntry};
    use crate::nexus::domain::entities::game::player_session::PlayerSession;
    use crate::nexus::domain::entities::game::server::{GameServer, GameServerStatus};
    use crate::nexus::domain::entities::system::bot_config::{BotDefinition, BotGuildConfig};
    use crate::nexus::ports::outbound::game::container_runtime::{
        ContainerSpec, ContainerStats, ContainerStatus, ManagedContainer,
    };
    use crate::nexus::ports::outbound::game::game_server_repository::{
        GameServerRuntimeUpdate, NewGameServer, TemplateUsage,
    };

    #[test]
    fn managed_server_label_supports_current_and_legacy_names() {
        let current = HashMap::from([("nexus.server_id".to_string(), "current".to_string())]);
        let legacy = HashMap::from([("sentinel.server_id".to_string(), "legacy".to_string())]);
        let both = HashMap::from([
            ("nexus.server_id".to_string(), "current".to_string()),
            ("sentinel.server_id".to_string(), "legacy".to_string()),
        ]);

        assert_eq!(managed_server_id_label(&current), Some("current"));
        assert_eq!(managed_server_id_label(&legacy), Some("legacy"));
        assert_eq!(managed_server_id_label(&both), Some("current"));
        assert_eq!(managed_server_id_label(&HashMap::new()), None);
    }

    struct DummyServerRepo;
    #[async_trait::async_trait]
    impl GameServerRepository for DummyServerRepo {
        async fn create(&self, _: NewGameServer) -> Result<GameServer, DomainError> {
            todo!()
        }
        async fn find_by_id(&self, _: Uuid) -> Result<Option<GameServer>, DomainError> {
            Ok(None)
        }
        async fn list_by_guild(&self, _: &str) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![])
        }
        async fn list_running(&self) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![])
        }
        async fn list_active(&self) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![])
        }
        async fn update_runtime(
            &self,
            _: Uuid,
            _: GameServerRuntimeUpdate,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn update_status(
            &self,
            _: Uuid,
            _: GameServerStatus,
            _: Option<&str>,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn try_transition_status(
            &self,
            _: Uuid,
            _: &[GameServerStatus],
            _: GameServerStatus,
        ) -> Result<bool, DomainError> {
            Ok(true)
        }
        async fn update_player_activity(&self, _: Uuid, _: i32) -> Result<(), DomainError> {
            Ok(())
        }
        async fn record_restart_attempt(&self, _: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn reset_restart_attempts(&self, _: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn soft_delete(&self, _: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn count_active_for_guild(&self, _: &str) -> Result<(i32, i32), DomainError> {
            Ok((0, 0))
        }
        async fn template_usages(
            &self,
            _: &[Uuid],
        ) -> Result<HashMap<Uuid, TemplateUsage>, DomainError> {
            Ok(HashMap::new())
        }
        async fn set_session_channels(
            &self,
            _: Uuid,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<bool, DomainError> {
            Ok(true)
        }
        async fn mark_ip_revealed(&self, _: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn list_ip_reveal_due(&self) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![])
        }
        async fn list_awaiting_reveal_no_ping_today(&self) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![])
        }
        async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn set_config_dirty(&self, _: uuid::Uuid, _: bool) -> Result<(), DomainError> {
            Ok(())
        }
        async fn set_closes_at(
            &self,
            _: uuid::Uuid,
            _: Option<chrono::DateTime<chrono::Utc>>,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn set_ip_reveal_at(
            &self,
            _: Uuid,
            _: Option<chrono::DateTime<chrono::Utc>>,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn list_scheduled_due_to_start(&self) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![])
        }
    }

    struct DummyEventPublisher;
    #[async_trait::async_trait]
    impl crate::nexus::ports::outbound::events::EventPublisher for DummyEventPublisher {
        async fn publish(&self, _: &str, _: serde_json::Value) {}
    }

    struct DummyBotConfig;
    #[async_trait::async_trait]
    impl BotConfigRepository for DummyBotConfig {
        async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
            Ok(vec![])
        }
        async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
            Ok(vec![])
        }
        async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
            Ok(vec![])
        }
        async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct DummyTemplateRepo;
    #[async_trait::async_trait]
    impl crate::nexus::ports::outbound::game::game_template_repository::GameTemplateRepository
        for DummyTemplateRepo
    {
        async fn list(
            &self,
        ) -> Result<Vec<crate::nexus::domain::entities::game::template::GameTemplate>, DomainError>
        {
            Ok(vec![])
        }
        async fn find_by_id(
            &self,
            _: Uuid,
        ) -> Result<Option<crate::nexus::domain::entities::game::template::GameTemplate>, DomainError>
        {
            Ok(None)
        }
        async fn find_by_slug(
            &self,
            _: &str,
        ) -> Result<Option<crate::nexus::domain::entities::game::template::GameTemplate>, DomainError>
        {
            Ok(None)
        }
    }

    struct DummyAuditRepo;
    #[async_trait::async_trait]
    impl GameAuditRepository for DummyAuditRepo {
        async fn log(
            &self,
            _: &str,
            _: Option<Uuid>,
            _: Option<&str>,
            _: GameAuditAction,
            _: serde_json::Value,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn list_for_server(
            &self,
            _: Uuid,
            _: i64,
        ) -> Result<Vec<GameAuditEntry>, DomainError> {
            Ok(vec![])
        }
        async fn list_for_guild(
            &self,
            _: &str,
            _: i64,
            _: i64,
        ) -> Result<Vec<GameAuditEntry>, DomainError> {
            Ok(vec![])
        }
    }

    struct DummySessionRepo;
    #[async_trait::async_trait]
    impl PlayerSessionRepository for DummySessionRepo {
        async fn open(&self, _: Uuid, _: &str) -> Result<Uuid, DomainError> {
            Ok(Uuid::new_v4())
        }
        async fn close(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn list_active(&self, _: Uuid) -> Result<Vec<PlayerSession>, DomainError> {
            Ok(vec![])
        }
        async fn list_history(
            &self,
            _: Uuid,
            _: i64,
            _: i64,
        ) -> Result<Vec<PlayerSession>, DomainError> {
            Ok(vec![])
        }
        async fn close_all_active(&self, _: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct DummyRuntime;
    #[async_trait::async_trait]
    impl ContainerRuntime for DummyRuntime {
        async fn ensure_network(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn ensure_volume(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn pull_image_if_missing(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn create_container(&self, _: &ContainerSpec) -> Result<String, DomainError> {
            Ok("id".into())
        }
        async fn start_container(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn upload_file_to_container(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn stop_container(&self, _: &str, _: u32) -> Result<(), DomainError> {
            Ok(())
        }
        async fn restart_container(&self, _: &str, _: u32) -> Result<(), DomainError> {
            Ok(())
        }
        async fn remove_container(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn remove_volume(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn remove_image(&self, _: &str, _: bool) -> Result<bool, DomainError> {
            Ok(true)
        }
        async fn inspect(&self, _: &str) -> Result<Option<ContainerStatus>, DomainError> {
            Ok(None)
        }
        async fn stats(&self, _: &str) -> Result<ContainerStats, DomainError> {
            todo!()
        }
        async fn logs(&self, _: &str, _: u32) -> Result<Vec<String>, DomainError> {
            Ok(vec![])
        }
        async fn list_managed_containers(&self) -> Result<Vec<ManagedContainer>, DomainError> {
            Ok(vec![])
        }
    }

    struct DummyRcon;
    #[async_trait::async_trait]
    impl RconClient for DummyRcon {
        async fn execute(
            &self,
            _: &RconConnectionParams,
            _: &str,
        ) -> Result<crate::nexus::ports::outbound::game::rcon_client::RconResponse, DomainError>
        {
            todo!()
        }
    }

    struct DummyPortAllocator;
    #[async_trait::async_trait]
    impl PortAllocator for DummyPortAllocator {
        async fn allocate(&self, _: PortKind, _: u16, _: u16, _: &str) -> Result<u16, DomainError> {
            Ok(25565)
        }
        async fn release(&self, _: PortKind, _: u16) -> Result<(), DomainError> {
            Ok(())
        }
        async fn is_available(&self, _: PortKind, _: u16) -> Result<bool, DomainError> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_run_reveal_ip_empty() {
        let ctx = JobContext {
            server_repo: Arc::new(DummyServerRepo),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(DummyRuntime),
            rcon_client: Arc::new(DummyRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(DummyBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_reveal_ip(&ctx).await.unwrap();
        assert_eq!(report.job, "reveal_ip");
        assert_eq!(report.processed, 0);
        assert_eq!(report.errors, 0);
    }

    #[tokio::test]
    async fn test_run_daily_ping_empty() {
        let ctx = JobContext {
            server_repo: Arc::new(DummyServerRepo),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(DummyRuntime),
            rcon_client: Arc::new(DummyRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(DummyBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_daily_ping(&ctx).await.unwrap();
        assert_eq!(report.job, "daily_ping");
        assert_eq!(report.processed, 0);
        assert_eq!(report.errors, 0);
    }
}
// Tâches métier du portail exécutées en arrière-plan. Chaque job reçoit ses
// dépendances par ports et doit rester relançable sans doublon.
