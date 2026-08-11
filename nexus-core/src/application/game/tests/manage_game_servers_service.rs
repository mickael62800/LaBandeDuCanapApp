use super::*;
use crate::domain::entities::game::audit::{GameAuditAction, GameAuditEntry};
use crate::domain::entities::game::template::PortProtocol as TemplatePortProtocol;
use crate::domain::entities::system::bot_config::{BotDefinition, BotGuildConfig};
use crate::ports::outbound::game::container_runtime::{
    ContainerRuntime, ContainerSpec, ContainerStats, ContainerStatus, ManagedContainer,
};
use crate::ports::outbound::game::game_audit_repository::GameAuditRepository;
use crate::ports::outbound::game::game_server_config_repository::GameServerConfigRepository;
use crate::ports::outbound::game::game_server_repository::{GameServerRepository, TemplateUsage};
use crate::ports::outbound::game::game_template_repository::GameTemplateRepository;
use crate::ports::outbound::game::port_allocator::{PortAllocator, PortKind};
use crate::ports::outbound::game::rcon_client::{RconClient, RconConnectionParams, RconResponse};
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[test]
fn test_render_template_placeholders() {
    let mut env = HashMap::new();
    env.insert("WORLD_NAME".to_string(), "MyWorld".to_string());
    env.insert("MAX_PLAYERS".to_string(), "10".to_string());

    let template = "Server name: {{ WORLD_NAME }}, max: {{MAX_PLAYERS}}, unset: {{ UNSET_VAR }}";
    let rendered = render_template(template, &env);

    assert_eq!(rendered, "Server name: MyWorld, max: 10, unset: ");
}

#[test]
fn test_render_template_unclosed() {
    let env = HashMap::new();
    let template = "Hello {{ UNCLOSED";
    let rendered = render_template(template, &env);

    assert_eq!(rendered, "Hello {{ UNCLOSED");
}

fn sample_template(slug: &str, protocol: TemplatePortProtocol, run_as_root: bool) -> GameTemplate {
    GameTemplate {
        id: Uuid::nil(),
        slug: slug.to_string(),
        name: slug.to_string(),
        description: None,
        image: "test:latest".to_string(),
        category: None,
        icon: None,
        accent_color: None,
        cover_image_url: None,
        container_port: 2456,
        port_protocol: protocol,
        volume_path: "/data".to_string(),
        run_as_root,
        default_memory_mb: 2048,
        min_memory_mb: 1024,
        max_memory_mb: 8192,
        default_env: serde_json::json!({
            "DEFAULT_VAR": "default_val",
            "NUMERIC_VAR": 8211,
            "BOOL_VAR": true
        }),
        config_schema: vec![],
        supports_rcon: false,
        supports_mods: false,
        idle_shutdown_days: 7,
        init_files: vec![],
        command: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn sample_server(host_port: Option<u16>) -> GameServer {
    GameServer {
        id: Uuid::nil(),
        guild_id: "guild_1".to_string(),
        template_id: Uuid::nil(),
        name: "Test Server".to_string(),
        status: GameServerStatus::Stopped,
        container_id: None,
        host_port,
        rcon_port: None,
        rcon_password: None,
        volume_name: None,
        container_name: None,
        allocated_memory_mb: 2048,
        cpu_limit: Some(4.0),
        owner_user_id: "user_1".to_string(),
        idle_shutdown_days: None,
        last_active_at: None,
        last_player_count: 0,
        restart_attempts: 0,
        last_restart_at: None,
        last_error: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        started_at: None,
        stopped_at: None,
        text_channel_id: None,
        voice_channel_id: None,
        ip_reveal_at: None,
        ip_revealed: false,
    }
}

fn sample_config() -> GamePortalConfig {
    GamePortalConfig {
        enabled: true,
        log_channel_id: None,
        max_servers_per_guild: 5,
        max_memory_total_mb: 16384,
        allowed_templates: vec!["minecraft-vanilla".to_string(), "valheim".to_string()],
        port_range_start: 25500,
        port_range_end: 25599,
        rcon_enabled: true,
        rcon_port_range_start: 25700,
        rcon_port_range_end: 25799,
        rcon_timeout_secs: 5,
        docker_network_name: "sentinel-games".to_string(),
        container_user: "1000:1000".to_string(),
        host_data_dir: "/var/lib/sentinel/games".to_string(),
        auto_create_world_volume: true,
        default_idle_shutdown_days: 7,
        auto_remove_unused_images: true,
        unused_image_grace_days: 7,
        restart_backoff_base_secs: 30,
        restart_backoff_cap_secs: 3600,
        stuck_transition_threshold_minutes: 10,
        stop_timeout_secs: 30,
        max_log_lines: 1000,
        auto_restart_on_crash: true,
        max_auto_restart_attempts: 3,
        session_category_id: None,
        ip_reveal_default_days: 7,
        session_daily_ping_enabled: false,
        session_daily_ping_hour: 18,
    }
}

#[test]
fn test_valheim_multi_port_mapping() {
    let dummy_service = ManageGameServersService {
        server_repo: Arc::new(DummyServerRepo),
        template_repo: Arc::new(DummyTemplateRepo),
        config_repo: Arc::new(DummyConfigRepo),
        audit_repo: Arc::new(DummyAuditRepo),
        port_allocator: Arc::new(DummyPortAllocator),
        container_runtime: Arc::new(DummyContainerRuntime),
        rcon_client: Arc::new(DummyRconClient),
        bot_config: Arc::new(DummyBotConfig),
    };

    let tmpl = sample_template("valheim", TemplatePortProtocol::Udp, true);
    let server = sample_server(Some(25500));
    let overrides = HashMap::new();
    let cfg = sample_config();

    let spec = dummy_service.build_spec(&server, &tmpl, &overrides, &cfg);

    assert_eq!(spec.port_mappings.len(), 3);
    assert_eq!(spec.port_mappings[0].host_port, 25500);
    assert_eq!(spec.port_mappings[0].container_port, 2456);
    assert_eq!(spec.port_mappings[0].protocol, PortProtocol::Udp);

    assert_eq!(spec.port_mappings[1].host_port, 25501);
    assert_eq!(spec.port_mappings[1].container_port, 2457);
    assert_eq!(spec.port_mappings[1].protocol, PortProtocol::Udp);

    assert_eq!(spec.port_mappings[2].host_port, 25502);
    assert_eq!(spec.port_mappings[2].container_port, 2458);
    assert_eq!(spec.port_mappings[2].protocol, PortProtocol::Udp);
}

#[test]
fn test_palworld_public_port_and_user_spec() {
    let dummy_service = ManageGameServersService {
        server_repo: Arc::new(DummyServerRepo),
        template_repo: Arc::new(DummyTemplateRepo),
        config_repo: Arc::new(DummyConfigRepo),
        audit_repo: Arc::new(DummyAuditRepo),
        port_allocator: Arc::new(DummyPortAllocator),
        container_runtime: Arc::new(DummyContainerRuntime),
        rcon_client: Arc::new(DummyRconClient),
        bot_config: Arc::new(DummyBotConfig),
    };

    let tmpl = sample_template("palworld", TemplatePortProtocol::Udp, false);
    let server = sample_server(Some(25505));
    let overrides = HashMap::new();
    let cfg = sample_config();

    let spec = dummy_service.build_spec(&server, &tmpl, &overrides, &cfg);

    assert_eq!(
        spec.env.get("PUBLIC_PORT").map(|s| s.as_str()),
        Some("25505")
    );
    assert_eq!(
        spec.env.get("NUMERIC_VAR").map(|s| s.as_str()),
        Some("8211")
    );
    assert_eq!(spec.env.get("BOOL_VAR").map(|s| s.as_str()), Some("true"));
    assert_eq!(spec.user, None);
}

#[test]
fn test_cpu_limit_clamping() {
    let mut cmd_over = sample_command();
    cmd_over.cpu_limit = Some(12.0);

    let clamped_cpu = cmd_over.cpu_limit.map(|c| c.clamp(0.5, 6.0));
    assert_eq!(clamped_cpu, Some(6.0));

    let mut cmd_under = sample_command();
    cmd_under.cpu_limit = Some(0.1);
    let clamped_under = cmd_under.cpu_limit.map(|c| c.clamp(0.5, 6.0));
    assert_eq!(clamped_under, Some(0.5));
}

fn sample_command() -> CreateGameServerCommand {
    CreateGameServerCommand {
        guild_id: "guild_1".to_string(),
        template_slug: "minecraft-vanilla".to_string(),
        name: "Test Server".to_string(),
        allocated_memory_mb: Some(2048),
        cpu_limit: Some(4.0),
        owner_user_id: "user_1".to_string(),
        initial_config: HashMap::new(),
    }
}

// Dummy structs for unit test dependency injection
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
    async fn list_active(&self) -> Result<Vec<GameServer>, DomainError> {
        Ok(vec![])
    }
    async fn list_running(&self) -> Result<Vec<GameServer>, DomainError> {
        Ok(vec![])
    }
    async fn update_runtime(&self, _: Uuid, _: GameServerRuntimeUpdate) -> Result<(), DomainError> {
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
    async fn record_restart_attempt(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn reset_restart_attempts(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn mark_ip_revealed(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update_player_activity(&self, _: Uuid, _: i32) -> Result<(), DomainError> {
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
    ) -> Result<std::collections::HashMap<Uuid, TemplateUsage>, DomainError> {
        Ok(std::collections::HashMap::new())
    }
    async fn set_session_channels(
        &self,
        _: Uuid,
        _: Option<&str>,
        _: Option<&str>,
    ) -> Result<bool, DomainError> {
        Ok(true)
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
    async fn set_ip_reveal_at(&self, _: Uuid, _: Option<DateTime<Utc>>) -> Result<(), DomainError> {
        Ok(())
    }
}

struct DummyTemplateRepo;
#[async_trait::async_trait]
impl GameTemplateRepository for DummyTemplateRepo {
    async fn find_by_id(&self, _: Uuid) -> Result<Option<GameTemplate>, DomainError> {
        Ok(None)
    }
    async fn find_by_slug(&self, _: &str) -> Result<Option<GameTemplate>, DomainError> {
        Ok(None)
    }
    async fn list(&self) -> Result<Vec<GameTemplate>, DomainError> {
        Ok(vec![])
    }
}

struct DummyConfigRepo;
#[async_trait::async_trait]
impl GameServerConfigRepository for DummyConfigRepo {
    async fn get_all(&self, _: Uuid) -> Result<HashMap<String, String>, DomainError> {
        Ok(HashMap::new())
    }
    async fn upsert(&self, _: Uuid, _: &str, _: &str, _: Option<&str>) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn replace_all(
        &self,
        _: Uuid,
        _: HashMap<String, String>,
        _: Option<&str>,
    ) -> Result<(), DomainError> {
        Ok(())
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
    async fn list_for_server(&self, _: Uuid, _: i64) -> Result<Vec<GameAuditEntry>, DomainError> {
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

struct DummyPortAllocator;
#[async_trait::async_trait]
impl PortAllocator for DummyPortAllocator {
    async fn allocate(&self, _: PortKind, _: u16, _: u16, _: &str) -> Result<u16, DomainError> {
        todo!()
    }
    async fn release(&self, _: PortKind, _: u16) -> Result<(), DomainError> {
        Ok(())
    }
    async fn is_available(&self, _: PortKind, _: u16) -> Result<bool, DomainError> {
        todo!()
    }
}

struct DummyContainerRuntime;
#[async_trait::async_trait]
impl ContainerRuntime for DummyContainerRuntime {
    async fn pull_image_if_missing(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn create_container(&self, _: &ContainerSpec) -> Result<String, DomainError> {
        Ok("cid".to_string())
    }
    async fn start_container(&self, _: &str) -> Result<(), DomainError> {
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
    async fn ensure_network(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn ensure_volume(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_volume(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_image(&self, _: &str, _: bool) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn upload_file_to_container(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn inspect(&self, _: &str) -> Result<Option<ContainerStatus>, DomainError> {
        todo!()
    }
    async fn list_managed_containers(&self) -> Result<Vec<ManagedContainer>, DomainError> {
        todo!()
    }
    async fn stats(&self, _: &str) -> Result<ContainerStats, DomainError> {
        todo!()
    }
    async fn logs(&self, _: &str, _: u32) -> Result<Vec<String>, DomainError> {
        Ok(vec![])
    }
}

struct DummyRconClient;
#[async_trait::async_trait]
impl RconClient for DummyRconClient {
    async fn execute(
        &self,
        _: &RconConnectionParams,
        _: &str,
    ) -> Result<RconResponse, DomainError> {
        todo!()
    }
}

struct DummyBotConfig;
#[async_trait::async_trait]
impl BotConfigRepository for DummyBotConfig {
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
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
