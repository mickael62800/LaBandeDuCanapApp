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
mod purge_history;
mod reconciler;
mod reveal_ip;

pub use daily_ping::run_daily_ping;
pub use health_check::run_health_check;
pub use idle_shutdown::run_idle_shutdown;
pub use image_cleanup::run_image_cleanup;
pub use purge_history::{run_purge_history, RETENTION_JOURS_DEFAUT};
pub use reconciler::run_reconciler;
pub use reveal_ip::run_reveal_ip;

#[cfg(test)]
mod tests {
    use super::*;
    // `VolumeArchive` ne sert qu'aux doubles de test qui implementent ContainerRuntime.
    use crate::nexus::domain::entities::game::audit::{GameAuditAction, GameAuditEntry};
    use crate::nexus::domain::entities::game::player_session::PlayerSession;
    use crate::nexus::domain::entities::game::server::{GameServer, GameServerStatus};
    use crate::nexus::domain::entities::system::bot_config::{BotDefinition, BotGuildConfig};
    use crate::nexus::ports::outbound::game::container_runtime::VolumeArchive;
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
        async fn compter_tentative_annonce(&self, _: uuid::Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn marquer_annonce_publiee(&self, _: uuid::Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn annonces_abandonnees(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![])
        }
        async fn marquer_abandon_signale(&self, _: uuid::Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn annonces_en_attente(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![])
        }
        async fn set_rules(&self, _: uuid::Uuid, _: Option<&str>) -> Result<(), DomainError> {
            Ok(())
        }
        async fn set_channel_names(
            &self,
            _: uuid::Uuid,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<(), DomainError> {
            Ok(())
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
        async fn update_resources(
            &self,
            _: uuid::Uuid,
            _: i32,
            _: Option<f64>,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn record_history(
            &self,
            _: uuid::Uuid,
            _: Option<f32>,
            _: Option<i32>,
            _: Option<i32>,
            _: Option<i32>,
            _: Option<i64>,
            _: Option<i64>,
            _: Option<i32>,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn history(
            &self,
            _: uuid::Uuid,
            _: i64,
            _: i64,
        ) -> Result<
            Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>,
            DomainError,
        > {
            Ok(vec![])
        }
        async fn purge_history(&self, _: i32) -> Result<u64, DomainError> {
            Ok(0)
        }
        async fn record_perf_sample(
            &self,
            _: uuid::Uuid,
            _: Option<i32>,
            _: Option<i64>,
            _: Option<i64>,
        ) -> Result<(), DomainError> {
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
        async fn count_history(&self, _: Uuid) -> Result<i64, DomainError> {
            Ok(0)
        }
        async fn close_all_active(&self, _: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct DummyRuntime;
    #[async_trait::async_trait]
    impl ContainerRuntime for DummyRuntime {
        async fn archive_volume(
            &self,
            _volume: &str,
            _nom_fichier: &str,
        ) -> Result<VolumeArchive, DomainError> {
            unimplemented!("archivage non couvert par ce double de test")
        }
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
            Ok(ContainerStats::default())
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

    #[tokio::test]
    async fn test_run_purge_history() {
        struct PurgeServerRepo;
        #[async_trait::async_trait]
        impl GameServerRepository for PurgeServerRepo {
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
            async fn compter_tentative_annonce(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn marquer_annonce_publiee(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_abandonnees(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn marquer_abandon_signale(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_en_attente(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn set_rules(&self, _: uuid::Uuid, _: Option<&str>) -> Result<(), DomainError> {
                Ok(())
            }
            async fn set_channel_names(
                &self,
                _: uuid::Uuid,
                _: Option<&str>,
                _: Option<&str>,
                _: Option<&str>,
            ) -> Result<(), DomainError> {
                Ok(())
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
            async fn list_awaiting_reveal_no_ping_today(
                &self,
            ) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn update_resources(
                &self,
                _: uuid::Uuid,
                _: i32,
                _: Option<f64>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn record_history(
                &self,
                _: uuid::Uuid,
                _: Option<f32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
                _: Option<i32>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn history(
                &self,
                _: uuid::Uuid,
                _: i64,
                _: i64,
            ) -> Result<
                Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>,
                DomainError,
            > {
                Ok(vec![])
            }
            async fn purge_history(&self, _: i32) -> Result<u64, DomainError> {
                Ok(42)
            }
            async fn record_perf_sample(
                &self,
                _: uuid::Uuid,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
            ) -> Result<(), DomainError> {
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

        let ctx = JobContext {
            server_repo: Arc::new(PurgeServerRepo),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(DummyRuntime),
            rcon_client: Arc::new(DummyRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(DummyBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_purge_history(&ctx, 7).await.unwrap();
        assert_eq!(report.job, "purge_history");
        assert_eq!(report.processed, 42);
    }

    #[tokio::test]
    async fn test_run_image_cleanup_flow() {
        struct ImageCleanupBotConfig;
        #[async_trait::async_trait]
        impl BotConfigRepository for ImageCleanupBotConfig {
            async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
                Ok(vec![])
            }
            async fn get_config(
                &self,
                _: &str,
                _: &str,
            ) -> Result<Vec<BotGuildConfig>, DomainError> {
                Ok(vec![
                    BotGuildConfig {
                        id: Uuid::new_v4(),
                        guild_id: "_global".into(),
                        bot_name: "game-portal".into(),
                        config_key: "auto_remove_unused_images".into(),
                        config_value: "true".into(),
                        updated_at: chrono::Utc::now(),
                    },
                    BotGuildConfig {
                        id: Uuid::new_v4(),
                        guild_id: "_global".into(),
                        bot_name: "game-portal".into(),
                        config_key: "unused_image_grace_days".into(),
                        config_value: "7".into(),
                        updated_at: chrono::Utc::now(),
                    },
                ])
            }
            async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
                Ok(vec![])
            }
            async fn set_config(
                &self,
                _: &str,
                _: &str,
                _: &str,
                _: &str,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
                Ok(())
            }
        }

        struct ImageCleanupTemplateRepo {
            tpl_id: Uuid,
        }
        #[async_trait::async_trait]
        impl crate::nexus::ports::outbound::game::game_template_repository::GameTemplateRepository
            for ImageCleanupTemplateRepo
        {
            async fn list(
                &self,
            ) -> Result<
                Vec<crate::nexus::domain::entities::game::template::GameTemplate>,
                DomainError,
            > {
                Ok(vec![
                    crate::nexus::domain::entities::game::template::GameTemplate {
                        id: self.tpl_id,
                        slug: "test-game".into(),
                        name: "Test Game".into(),
                        description: None,
                        image: "test-game:latest".into(),
                        category: None,
                        icon: None,
                        accent_color: None,
                        cover_image_url: None,
                        container_port: 25565,
                        port_protocol:
                            crate::nexus::domain::entities::game::template::PortProtocol::Tcp,
                        extra_ports: vec![],
                        volume_path: "/data".into(),
                        run_as_root: false,
                        default_memory_mb: 1024,
                        min_memory_mb: 512,
                        max_memory_mb: 2048,
                        default_env: serde_json::json!({}),
                        config_schema: vec![],
                        command_schema: vec![],
                        supports_rcon: false,
                        supports_mods: false,
                        idle_shutdown_days: 7,
                        init_files: vec![],
                        command: None,
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                    },
                ])
            }
            async fn find_by_id(
                &self,
                _: Uuid,
            ) -> Result<
                Option<crate::nexus::domain::entities::game::template::GameTemplate>,
                DomainError,
            > {
                Ok(None)
            }
            async fn find_by_slug(
                &self,
                _: &str,
            ) -> Result<
                Option<crate::nexus::domain::entities::game::template::GameTemplate>,
                DomainError,
            > {
                Ok(None)
            }
        }

        struct ImageCleanupServerRepo {
            tpl_id: Uuid,
        }
        #[async_trait::async_trait]
        impl GameServerRepository for ImageCleanupServerRepo {
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
                let mut m = HashMap::new();
                m.insert(
                    self.tpl_id,
                    TemplateUsage {
                        active_count: 0,
                        last_activity_at: Some(chrono::Utc::now() - chrono::Duration::days(10)),
                    },
                );
                Ok(m)
            }
            async fn compter_tentative_annonce(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn marquer_annonce_publiee(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_abandonnees(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn marquer_abandon_signale(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_en_attente(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn set_rules(&self, _: uuid::Uuid, _: Option<&str>) -> Result<(), DomainError> {
                Ok(())
            }
            async fn set_channel_names(
                &self,
                _: uuid::Uuid,
                _: Option<&str>,
                _: Option<&str>,
                _: Option<&str>,
            ) -> Result<(), DomainError> {
                Ok(())
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
            async fn list_awaiting_reveal_no_ping_today(
                &self,
            ) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn update_resources(
                &self,
                _: uuid::Uuid,
                _: i32,
                _: Option<f64>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn record_history(
                &self,
                _: uuid::Uuid,
                _: Option<f32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
                _: Option<i32>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn history(
                &self,
                _: uuid::Uuid,
                _: i64,
                _: i64,
            ) -> Result<
                Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>,
                DomainError,
            > {
                Ok(vec![])
            }
            async fn purge_history(&self, _: i32) -> Result<u64, DomainError> {
                Ok(0)
            }
            async fn record_perf_sample(
                &self,
                _: uuid::Uuid,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
            ) -> Result<(), DomainError> {
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

        let tpl_id = Uuid::new_v4();
        let ctx = JobContext {
            server_repo: Arc::new(ImageCleanupServerRepo { tpl_id }),
            template_repo: Arc::new(ImageCleanupTemplateRepo { tpl_id }),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(DummyRuntime),
            rcon_client: Arc::new(DummyRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(ImageCleanupBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_image_cleanup(&ctx).await.unwrap();
        assert_eq!(report.job, "image_cleanup");
        assert_eq!(report.processed, 1);
    }

    #[tokio::test]
    async fn test_run_idle_shutdown_flow() {
        struct IdleServerRepo {
            server: GameServer,
        }
        #[async_trait::async_trait]
        impl GameServerRepository for IdleServerRepo {
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
                Ok(vec![self.server.clone()])
            }
            async fn list_active(&self) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![self.server.clone()])
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
            async fn compter_tentative_annonce(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn marquer_annonce_publiee(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_abandonnees(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn marquer_abandon_signale(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_en_attente(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn set_rules(&self, _: uuid::Uuid, _: Option<&str>) -> Result<(), DomainError> {
                Ok(())
            }
            async fn set_channel_names(
                &self,
                _: uuid::Uuid,
                _: Option<&str>,
                _: Option<&str>,
                _: Option<&str>,
            ) -> Result<(), DomainError> {
                Ok(())
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
            async fn list_awaiting_reveal_no_ping_today(
                &self,
            ) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn update_resources(
                &self,
                _: uuid::Uuid,
                _: i32,
                _: Option<f64>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn record_history(
                &self,
                _: uuid::Uuid,
                _: Option<f32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
                _: Option<i32>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn history(
                &self,
                _: uuid::Uuid,
                _: i64,
                _: i64,
            ) -> Result<
                Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>,
                DomainError,
            > {
                Ok(vec![])
            }
            async fn purge_history(&self, _: i32) -> Result<u64, DomainError> {
                Ok(0)
            }
            async fn record_perf_sample(
                &self,
                _: uuid::Uuid,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
            ) -> Result<(), DomainError> {
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

        let s = GameServer {
            id: Uuid::new_v4(),
            guild_id: "guild_1".into(),
            template_id: Uuid::new_v4(),
            name: "Idle Server".into(),
            status: GameServerStatus::Running,
            container_id: Some("cid123".into()),
            host_port: Some(25565),
            rcon_port: Some(25575),
            rcon_password: Some("pwd".into()),
            volume_name: None,
            container_name: None,
            allocated_memory_mb: 2048,
            cpu_limit: None,
            owner_user_id: "u1".into(),
            idle_shutdown_days: Some(3),
            last_active_at: Some(chrono::Utc::now() - chrono::Duration::days(5)),
            last_player_count: 0,
            restart_attempts: 0,
            last_restart_at: None,
            last_error: None,
            created_at: chrono::Utc::now() - chrono::Duration::days(10),
            updated_at: chrono::Utc::now(),
            started_at: None,
            stopped_at: None,
            announcement_posted_at: None,
            announcement_attempts: 0,
            announcement_abandon_notified_at: None,
            rules: None,
            channel_name_registration: None,
            channel_name_private: None,
            channel_name_voice: None,
            text_channel_id: None,
            voice_channel_id: None,
            ip_reveal_at: None,
            ip_revealed: false,
            closes_at: None,
            config_dirty: false,
            rcon_latency_ms: None,
            net_rx_bytes: None,
            net_tx_bytes: None,
            net_sampled_at: None,
        };

        let ctx = JobContext {
            server_repo: Arc::new(IdleServerRepo { server: s }),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(DummyRuntime),
            rcon_client: Arc::new(DummyRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(DummyBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_idle_shutdown(&ctx).await.unwrap();
        assert_eq!(report.job, "idle_shutdown");
        assert_eq!(report.processed, 1);
    }

    #[tokio::test]
    async fn test_run_reconciler_and_health_check() {
        struct HealthRcon;
        #[async_trait::async_trait]
        impl RconClient for HealthRcon {
            async fn execute(
                &self,
                _: &RconConnectionParams,
                _: &str,
            ) -> Result<crate::nexus::ports::outbound::game::rcon_client::RconResponse, DomainError>
            {
                Ok(
                    crate::nexus::ports::outbound::game::rcon_client::RconResponse {
                        raw: "There are 2 of a max of 20 players online: PlayerOne, PlayerTwo"
                            .into(),
                    },
                )
            }
        }

        struct HealthServerRepo {
            server: GameServer,
        }
        #[async_trait::async_trait]
        impl GameServerRepository for HealthServerRepo {
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
                Ok(vec![self.server.clone()])
            }
            async fn list_active(&self) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![self.server.clone()])
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
            async fn compter_tentative_annonce(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn marquer_annonce_publiee(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_abandonnees(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn marquer_abandon_signale(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_en_attente(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn set_rules(&self, _: uuid::Uuid, _: Option<&str>) -> Result<(), DomainError> {
                Ok(())
            }
            async fn set_channel_names(
                &self,
                _: uuid::Uuid,
                _: Option<&str>,
                _: Option<&str>,
                _: Option<&str>,
            ) -> Result<(), DomainError> {
                Ok(())
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
            async fn list_awaiting_reveal_no_ping_today(
                &self,
            ) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn update_resources(
                &self,
                _: uuid::Uuid,
                _: i32,
                _: Option<f64>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn record_history(
                &self,
                _: uuid::Uuid,
                _: Option<f32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
                _: Option<i32>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn history(
                &self,
                _: uuid::Uuid,
                _: i64,
                _: i64,
            ) -> Result<
                Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>,
                DomainError,
            > {
                Ok(vec![])
            }
            async fn purge_history(&self, _: i32) -> Result<u64, DomainError> {
                Ok(0)
            }
            async fn record_perf_sample(
                &self,
                _: uuid::Uuid,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
            ) -> Result<(), DomainError> {
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

        let s = GameServer {
            id: Uuid::new_v4(),
            guild_id: "guild_1".into(),
            template_id: Uuid::new_v4(),
            name: "Health Server".into(),
            status: GameServerStatus::Running,
            container_id: Some("cid123".into()),
            host_port: Some(25565),
            rcon_port: Some(25575),
            rcon_password: Some("pwd".into()),
            volume_name: None,
            container_name: None,
            allocated_memory_mb: 2048,
            cpu_limit: None,
            owner_user_id: "u1".into(),
            idle_shutdown_days: Some(3),
            last_active_at: Some(chrono::Utc::now()),
            last_player_count: 0,
            restart_attempts: 0,
            last_restart_at: None,
            last_error: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            started_at: None,
            stopped_at: None,
            announcement_posted_at: None,
            announcement_attempts: 0,
            announcement_abandon_notified_at: None,
            rules: None,
            channel_name_registration: None,
            channel_name_private: None,
            channel_name_voice: None,
            text_channel_id: None,
            voice_channel_id: None,
            ip_reveal_at: None,
            ip_revealed: false,
            closes_at: None,
            config_dirty: false,
            rcon_latency_ms: None,
            net_rx_bytes: None,
            net_tx_bytes: None,
            net_sampled_at: None,
        };

        let ctx = JobContext {
            server_repo: Arc::new(HealthServerRepo { server: s }),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(DummyRuntime),
            rcon_client: Arc::new(HealthRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(DummyBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_health_check(&ctx).await.unwrap();
        assert_eq!(report.job, "health_check");
        assert_eq!(report.processed, 1);

        let report_rec = run_reconciler(&ctx).await.unwrap();
        assert_eq!(report_rec.job, "reconciler");
    }

    #[tokio::test]
    async fn test_run_reveal_ip_with_due_servers() {
        struct DueServerRepo {
            server: GameServer,
        }
        #[async_trait::async_trait]
        impl GameServerRepository for DueServerRepo {
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
            async fn compter_tentative_annonce(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn marquer_annonce_publiee(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_abandonnees(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn marquer_abandon_signale(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_en_attente(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn set_rules(&self, _: uuid::Uuid, _: Option<&str>) -> Result<(), DomainError> {
                Ok(())
            }
            async fn set_channel_names(
                &self,
                _: uuid::Uuid,
                _: Option<&str>,
                _: Option<&str>,
                _: Option<&str>,
            ) -> Result<(), DomainError> {
                Ok(())
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
                Ok(vec![self.server.clone()])
            }
            async fn list_awaiting_reveal_no_ping_today(
                &self,
            ) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn update_resources(
                &self,
                _: uuid::Uuid,
                _: i32,
                _: Option<f64>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn record_history(
                &self,
                _: uuid::Uuid,
                _: Option<f32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
                _: Option<i32>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn history(
                &self,
                _: uuid::Uuid,
                _: i64,
                _: i64,
            ) -> Result<
                Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>,
                DomainError,
            > {
                Ok(vec![])
            }
            async fn purge_history(&self, _: i32) -> Result<u64, DomainError> {
                Ok(0)
            }
            async fn record_perf_sample(
                &self,
                _: uuid::Uuid,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
            ) -> Result<(), DomainError> {
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

        struct RevealBotConfig;
        #[async_trait::async_trait]
        impl BotConfigRepository for RevealBotConfig {
            async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
                Ok(vec![])
            }
            async fn get_config(
                &self,
                _: &str,
                _: &str,
            ) -> Result<Vec<BotGuildConfig>, DomainError> {
                Ok(vec![BotGuildConfig {
                    id: Uuid::new_v4(),
                    guild_id: "guild_1".into(),
                    bot_name: "game-portal".into(),
                    config_key: "session_public_host".into(),
                    config_value: "play.example.com".into(),
                    updated_at: chrono::Utc::now(),
                }])
            }
            async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
                Ok(vec![])
            }
            async fn set_config(
                &self,
                _: &str,
                _: &str,
                _: &str,
                _: &str,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
                Ok(())
            }
        }

        let mut s = sample_server_for_jobs();
        s.host_port = Some(25565);
        let ctx = JobContext {
            server_repo: Arc::new(DueServerRepo { server: s }),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(DummyRuntime),
            rcon_client: Arc::new(DummyRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(RevealBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_reveal_ip(&ctx).await.unwrap();
        assert_eq!(report.job, "reveal_ip");
        assert_eq!(report.processed, 1);
    }

    #[tokio::test]
    async fn test_run_daily_ping_with_servers() {
        struct AwaitingServerRepo {
            server: GameServer,
        }
        #[async_trait::async_trait]
        impl GameServerRepository for AwaitingServerRepo {
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
            async fn compter_tentative_annonce(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn marquer_annonce_publiee(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_abandonnees(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn marquer_abandon_signale(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_en_attente(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn set_rules(&self, _: uuid::Uuid, _: Option<&str>) -> Result<(), DomainError> {
                Ok(())
            }
            async fn set_channel_names(
                &self,
                _: uuid::Uuid,
                _: Option<&str>,
                _: Option<&str>,
                _: Option<&str>,
            ) -> Result<(), DomainError> {
                Ok(())
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
            async fn list_awaiting_reveal_no_ping_today(
                &self,
            ) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![self.server.clone()])
            }
            async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn update_resources(
                &self,
                _: uuid::Uuid,
                _: i32,
                _: Option<f64>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn record_history(
                &self,
                _: uuid::Uuid,
                _: Option<f32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
                _: Option<i32>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn history(
                &self,
                _: uuid::Uuid,
                _: i64,
                _: i64,
            ) -> Result<
                Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>,
                DomainError,
            > {
                Ok(vec![])
            }
            async fn purge_history(&self, _: i32) -> Result<u64, DomainError> {
                Ok(0)
            }
            async fn record_perf_sample(
                &self,
                _: uuid::Uuid,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
            ) -> Result<(), DomainError> {
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

        struct PingBotConfig;
        #[async_trait::async_trait]
        impl BotConfigRepository for PingBotConfig {
            async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
                Ok(vec![])
            }
            async fn get_config(
                &self,
                _: &str,
                _: &str,
            ) -> Result<Vec<BotGuildConfig>, DomainError> {
                Ok(vec![
                    BotGuildConfig {
                        id: Uuid::new_v4(),
                        guild_id: "guild_1".into(),
                        bot_name: "game-portal".into(),
                        config_key: "session_daily_ping_enabled".into(),
                        config_value: "true".into(),
                        updated_at: chrono::Utc::now(),
                    },
                    BotGuildConfig {
                        id: Uuid::new_v4(),
                        guild_id: "guild_1".into(),
                        bot_name: "game-portal".into(),
                        config_key: "session_daily_ping_hour".into(),
                        config_value: "0".into(),
                        updated_at: chrono::Utc::now(),
                    },
                ])
            }
            async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
                Ok(vec![])
            }
            async fn set_config(
                &self,
                _: &str,
                _: &str,
                _: &str,
                _: &str,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
                Ok(())
            }
        }

        let s = sample_server_for_jobs();
        let ctx = JobContext {
            server_repo: Arc::new(AwaitingServerRepo { server: s }),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(DummyRuntime),
            rcon_client: Arc::new(DummyRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(PingBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_daily_ping(&ctx).await.unwrap();
        assert_eq!(report.job, "daily_ping");
        assert_eq!(report.processed, 1);
    }

    #[tokio::test]
    async fn test_idle_shutdown_all_branches() {
        // Cas 1: days <= 0
        let mut s1 = sample_server_for_jobs();
        s1.idle_shutdown_days = Some(0);

        // Cas 2: pas de rcon
        let mut s2 = sample_server_for_jobs();
        s2.rcon_port = None;

        // Cas 3: last_player_count > 0
        let mut s3 = sample_server_for_jobs();
        s3.last_player_count = 3;
        s3.last_active_at = Some(chrono::Utc::now() - chrono::Duration::days(10));

        // Cas 4: activite recente
        let mut s4 = sample_server_for_jobs();
        s4.last_active_at = Some(chrono::Utc::now());

        struct MultiServerRepo {
            servers: Vec<GameServer>,
        }
        #[async_trait::async_trait]
        impl GameServerRepository for MultiServerRepo {
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
                Ok(self.servers.clone())
            }
            async fn list_active(&self) -> Result<Vec<GameServer>, DomainError> {
                Ok(self.servers.clone())
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
            async fn compter_tentative_annonce(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn marquer_annonce_publiee(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_abandonnees(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn marquer_abandon_signale(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_en_attente(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn set_rules(&self, _: uuid::Uuid, _: Option<&str>) -> Result<(), DomainError> {
                Ok(())
            }
            async fn set_channel_names(
                &self,
                _: uuid::Uuid,
                _: Option<&str>,
                _: Option<&str>,
                _: Option<&str>,
            ) -> Result<(), DomainError> {
                Ok(())
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
            async fn list_awaiting_reveal_no_ping_today(
                &self,
            ) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn update_resources(
                &self,
                _: uuid::Uuid,
                _: i32,
                _: Option<f64>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn record_history(
                &self,
                _: uuid::Uuid,
                _: Option<f32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
                _: Option<i32>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn history(
                &self,
                _: uuid::Uuid,
                _: i64,
                _: i64,
            ) -> Result<
                Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>,
                DomainError,
            > {
                Ok(vec![])
            }
            async fn purge_history(&self, _: i32) -> Result<u64, DomainError> {
                Ok(0)
            }
            async fn record_perf_sample(
                &self,
                _: uuid::Uuid,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
            ) -> Result<(), DomainError> {
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

        let ctx = JobContext {
            server_repo: Arc::new(MultiServerRepo {
                servers: vec![s1, s2, s3, s4],
            }),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(DummyRuntime),
            rcon_client: Arc::new(DummyRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(DummyBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_idle_shutdown(&ctx).await.unwrap();
        assert_eq!(report.job, "idle_shutdown");
        assert_eq!(report.processed, 0);
    }

    struct RecRuntime {
        containers: Vec<ManagedContainer>,
    }
    #[async_trait::async_trait]
    impl ContainerRuntime for RecRuntime {
        async fn archive_volume(
            &self,
            _volume: &str,
            _nom_fichier: &str,
        ) -> Result<VolumeArchive, DomainError> {
            unimplemented!("archivage non couvert par ce double de test")
        }

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
            Ok(ContainerStats::default())
        }
        async fn logs(&self, _: &str, _: u32) -> Result<Vec<String>, DomainError> {
            Ok(vec![])
        }
        async fn list_managed_containers(&self) -> Result<Vec<ManagedContainer>, DomainError> {
            Ok(self.containers.clone())
        }
    }

    #[tokio::test]
    async fn test_reconciler_all_branches() {
        let mut s_starting = sample_server_for_jobs();
        s_starting.status = GameServerStatus::Starting;
        s_starting.updated_at = chrono::Utc::now() - chrono::Duration::minutes(20);

        let mut s_stopping = sample_server_for_jobs();
        s_stopping.status = GameServerStatus::Stopping;
        s_stopping.updated_at = chrono::Utc::now() - chrono::Duration::minutes(20);

        let mut s_running_no_c = sample_server_for_jobs();
        s_running_no_c.status = GameServerStatus::Running;
        s_running_no_c.id = Uuid::new_v4();

        struct RecRepo {
            servers: Vec<GameServer>,
        }
        #[async_trait::async_trait]
        impl GameServerRepository for RecRepo {
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
                Ok(self.servers.clone())
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
            async fn compter_tentative_annonce(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn marquer_annonce_publiee(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_abandonnees(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn marquer_abandon_signale(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_en_attente(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn set_rules(&self, _: uuid::Uuid, _: Option<&str>) -> Result<(), DomainError> {
                Ok(())
            }
            async fn set_channel_names(
                &self,
                _: uuid::Uuid,
                _: Option<&str>,
                _: Option<&str>,
                _: Option<&str>,
            ) -> Result<(), DomainError> {
                Ok(())
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
            async fn list_awaiting_reveal_no_ping_today(
                &self,
            ) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn update_resources(
                &self,
                _: uuid::Uuid,
                _: i32,
                _: Option<f64>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn record_history(
                &self,
                _: uuid::Uuid,
                _: Option<f32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
                _: Option<i32>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn history(
                &self,
                _: uuid::Uuid,
                _: i64,
                _: i64,
            ) -> Result<
                Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>,
                DomainError,
            > {
                Ok(vec![])
            }
            async fn purge_history(&self, _: i32) -> Result<u64, DomainError> {
                Ok(0)
            }
            async fn record_perf_sample(
                &self,
                _: uuid::Uuid,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
            ) -> Result<(), DomainError> {
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

        let orphan = ManagedContainer {
            container_id: "orphan_cid".into(),
            name: "orphan".into(),
            state: ContainerState::Running,
            labels: HashMap::from([("nexus.server_id".to_string(), Uuid::new_v4().to_string())]),
        };

        let ctx = JobContext {
            server_repo: Arc::new(RecRepo {
                servers: vec![s_starting, s_stopping, s_running_no_c],
            }),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(RecRuntime {
                containers: vec![orphan],
            }),
            rcon_client: Arc::new(DummyRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(DummyBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_reconciler(&ctx).await.unwrap();
        assert_eq!(report.job, "reconciler");
    }

    #[tokio::test]
    async fn test_health_check_various_scenarios() {
        let mut s_no_rcon = sample_server_for_jobs();
        s_no_rcon.rcon_port = None;

        let mut s_restart_attempts = sample_server_for_jobs();
        s_restart_attempts.restart_attempts = 2;

        let mut s_rcon_err = sample_server_for_jobs();
        s_rcon_err.id = Uuid::new_v4();

        struct ErrRcon;
        #[async_trait::async_trait]
        impl RconClient for ErrRcon {
            async fn execute(
                &self,
                _: &RconConnectionParams,
                _: &str,
            ) -> Result<crate::nexus::ports::outbound::game::rcon_client::RconResponse, DomainError>
            {
                Err(DomainError::Internal("rcon timeout".into()))
            }
        }

        struct HCRepo {
            servers: Vec<GameServer>,
        }
        #[async_trait::async_trait]
        impl GameServerRepository for HCRepo {
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
                Ok(self.servers.clone())
            }
            async fn list_active(&self) -> Result<Vec<GameServer>, DomainError> {
                Ok(self.servers.clone())
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
            async fn compter_tentative_annonce(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn marquer_annonce_publiee(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_abandonnees(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn marquer_abandon_signale(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_en_attente(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn set_rules(&self, _: uuid::Uuid, _: Option<&str>) -> Result<(), DomainError> {
                Ok(())
            }
            async fn set_channel_names(
                &self,
                _: uuid::Uuid,
                _: Option<&str>,
                _: Option<&str>,
                _: Option<&str>,
            ) -> Result<(), DomainError> {
                Ok(())
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
            async fn list_awaiting_reveal_no_ping_today(
                &self,
            ) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn update_resources(
                &self,
                _: uuid::Uuid,
                _: i32,
                _: Option<f64>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn record_history(
                &self,
                _: uuid::Uuid,
                _: Option<f32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
                _: Option<i32>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn history(
                &self,
                _: uuid::Uuid,
                _: i64,
                _: i64,
            ) -> Result<
                Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>,
                DomainError,
            > {
                Ok(vec![])
            }
            async fn purge_history(&self, _: i32) -> Result<u64, DomainError> {
                Ok(0)
            }
            async fn record_perf_sample(
                &self,
                _: uuid::Uuid,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
            ) -> Result<(), DomainError> {
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

        let ctx = JobContext {
            server_repo: Arc::new(HCRepo {
                servers: vec![s_no_rcon, s_restart_attempts, s_rcon_err],
            }),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(DummyRuntime),
            rcon_client: Arc::new(ErrRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(DummyBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_health_check(&ctx).await.unwrap();
        assert_eq!(report.job, "health_check");
    }

    #[tokio::test]
    async fn test_image_cleanup_edge_cases() {
        struct DisabledImageBotConfig;
        #[async_trait::async_trait]
        impl BotConfigRepository for DisabledImageBotConfig {
            async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
                Ok(vec![])
            }
            async fn get_config(
                &self,
                _: &str,
                _: &str,
            ) -> Result<Vec<BotGuildConfig>, DomainError> {
                Ok(vec![BotGuildConfig {
                    id: Uuid::new_v4(),
                    guild_id: "_global".into(),
                    bot_name: "game-portal".into(),
                    config_key: "auto_remove_unused_images".into(),
                    config_value: "false".into(),
                    updated_at: chrono::Utc::now(),
                }])
            }
            async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
                Ok(vec![])
            }
            async fn set_config(
                &self,
                _: &str,
                _: &str,
                _: &str,
                _: &str,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
                Ok(())
            }
        }

        let ctx = JobContext {
            server_repo: Arc::new(DummyServerRepo),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(DummyRuntime),
            rcon_client: Arc::new(DummyRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(DisabledImageBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_image_cleanup(&ctx).await.unwrap();
        assert_eq!(report.job, "image_cleanup");
        assert_eq!(report.processed, 0);
    }

    #[tokio::test]
    async fn test_reconciler_running_crash_and_restart() {
        let mut s_crash = sample_server_for_jobs();
        s_crash.status = GameServerStatus::Running;
        s_crash.restart_attempts = 0;

        struct CrashRepo {
            server: GameServer,
        }
        #[async_trait::async_trait]
        impl GameServerRepository for CrashRepo {
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
                Ok(vec![self.server.clone()])
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
            async fn compter_tentative_annonce(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn marquer_annonce_publiee(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_abandonnees(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn marquer_abandon_signale(&self, _: uuid::Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn annonces_en_attente(&self, _: i32) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn set_rules(&self, _: uuid::Uuid, _: Option<&str>) -> Result<(), DomainError> {
                Ok(())
            }
            async fn set_channel_names(
                &self,
                _: uuid::Uuid,
                _: Option<&str>,
                _: Option<&str>,
                _: Option<&str>,
            ) -> Result<(), DomainError> {
                Ok(())
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
            async fn list_awaiting_reveal_no_ping_today(
                &self,
            ) -> Result<Vec<GameServer>, DomainError> {
                Ok(vec![])
            }
            async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn update_resources(
                &self,
                _: uuid::Uuid,
                _: i32,
                _: Option<f64>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn record_history(
                &self,
                _: uuid::Uuid,
                _: Option<f32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
                _: Option<i32>,
            ) -> Result<(), DomainError> {
                Ok(())
            }
            async fn history(
                &self,
                _: uuid::Uuid,
                _: i64,
                _: i64,
            ) -> Result<
                Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>,
                DomainError,
            > {
                Ok(vec![])
            }
            async fn purge_history(&self, _: i32) -> Result<u64, DomainError> {
                Ok(0)
            }
            async fn record_perf_sample(
                &self,
                _: uuid::Uuid,
                _: Option<i32>,
                _: Option<i64>,
                _: Option<i64>,
            ) -> Result<(), DomainError> {
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

        let dead_c = ManagedContainer {
            container_id: "cid123".into(),
            name: "dead_c".into(),
            state: ContainerState::Exited,
            labels: HashMap::from([("nexus.server_id".to_string(), s_crash.id.to_string())]),
        };

        let ctx = JobContext {
            server_repo: Arc::new(CrashRepo { server: s_crash }),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(RecRuntime {
                containers: vec![dead_c],
            }),
            rcon_client: Arc::new(DummyRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(DummyBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_reconciler(&ctx).await.unwrap();
        assert_eq!(report.job, "reconciler");
    }

    fn sample_server_for_jobs() -> GameServer {
        GameServer {
            id: Uuid::new_v4(),
            guild_id: "guild_1".into(),
            template_id: Uuid::new_v4(),
            name: "Server".into(),
            status: GameServerStatus::Running,
            container_id: Some("cid123".into()),
            host_port: Some(25565),
            rcon_port: Some(25575),
            rcon_password: Some("pwd".into()),
            volume_name: None,
            container_name: None,
            allocated_memory_mb: 2048,
            cpu_limit: None,
            owner_user_id: "u1".into(),
            idle_shutdown_days: Some(3),
            last_active_at: Some(chrono::Utc::now()),
            last_player_count: 0,
            restart_attempts: 0,
            last_restart_at: None,
            last_error: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            started_at: None,
            stopped_at: None,
            announcement_posted_at: None,
            announcement_attempts: 0,
            announcement_abandon_notified_at: None,
            rules: None,
            channel_name_registration: None,
            channel_name_private: None,
            channel_name_voice: None,
            text_channel_id: None,
            voice_channel_id: None,
            ip_reveal_at: None,
            ip_revealed: false,
            closes_at: None,
            config_dirty: false,
            rcon_latency_ms: None,
            net_rx_bytes: None,
            net_tx_bytes: None,
            net_sampled_at: None,
        }
    }
}

// Tâches métier du portail exécutées en arrière-plan. Chaque job reçoit ses
// dépendances par ports et doit rester relançable sans doublon.
