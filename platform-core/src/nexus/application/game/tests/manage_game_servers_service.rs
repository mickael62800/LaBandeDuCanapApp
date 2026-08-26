use super::provisioning::render_template;
use super::*;
use crate::nexus::domain::entities::game::audit::{GameAuditAction, GameAuditEntry};
use crate::nexus::domain::entities::game::template::{
    ExtraPort, PortProtocol as TemplatePortProtocol,
};
use crate::nexus::domain::entities::system::bot_config::{BotDefinition, BotGuildConfig};
use crate::nexus::ports::outbound::game::container_runtime::{
    ContainerRuntime, ContainerSpec, ContainerState, ContainerStats, ContainerStatus,
    ManagedContainer, VolumeArchive,
};
use crate::nexus::ports::outbound::game::game_audit_repository::GameAuditRepository;
use crate::nexus::ports::outbound::game::game_server_config_repository::GameServerConfigRepository;
use crate::nexus::ports::outbound::game::game_server_repository::{
    GameServerRepository, TemplateUsage,
};
use crate::nexus::ports::outbound::game::game_template_repository::GameTemplateRepository;
use crate::nexus::ports::outbound::game::port_allocator::{PortAllocator, PortKind};
use crate::nexus::ports::outbound::game::rcon_client::{
    RconClient, RconConnectionParams, RconResponse,
};
use crate::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Mutex;

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
        extra_ports: vec![],
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
        command_schema: vec![],
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
        backup_on_restart: true,
        backup_min_interval_hours: 24,
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

    // Les ports additionnels viennent du CATALOGUE, plus du slug : c'est la
    // fiche du jeu qui declare +1 et +2 en UDP (migration 053).
    let mut tmpl = sample_template("valheim", TemplatePortProtocol::Udp, true);
    tmpl.extra_ports = vec![
        ExtraPort {
            offset: 1,
            protocol: TemplatePortProtocol::Udp,
        },
        ExtraPort {
            offset: 2,
            protocol: TemplatePortProtocol::Udp,
        },
    ];
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
fn un_decalage_nul_publie_le_meme_port_dans_l_autre_protocole() {
    // Vintage Story ecoute sur 42420 en TCP ET en UDP. Sans ce cas, le jeu
    // demarrait avec une seule des deux ouvertures : les clients se
    // connectaient, puis perdaient le serveur.
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

    let mut tmpl = sample_template("vintage-story", TemplatePortProtocol::Tcp, false);
    tmpl.extra_ports = vec![ExtraPort {
        offset: 0,
        protocol: TemplatePortProtocol::Udp,
    }];
    let spec = dummy_service.build_spec(
        &sample_server(Some(25510)),
        &tmpl,
        &HashMap::new(),
        &sample_config(),
    );

    assert_eq!(spec.port_mappings.len(), 2);
    assert_eq!(spec.port_mappings[0].host_port, 25510);
    assert_eq!(spec.port_mappings[0].protocol, PortProtocol::Tcp);
    assert_eq!(spec.port_mappings[1].host_port, 25510);
    assert_eq!(spec.port_mappings[1].container_port, 2456);
    assert_eq!(spec.port_mappings[1].protocol, PortProtocol::Udp);
}

#[test]
fn ark_recoit_son_mot_de_passe_de_console_sans_variable_inventee() {
    // L'image ARK ouvre RCON d'elle-meme : il n'existe aucune variable
    // d'activation. En poser une donnerait, dans la configuration du
    // conteneur, l'apparence d'un reglage qui commande la console alors que
    // rien ne le lit.
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

    let mut tmpl = sample_template("ark", TemplatePortProtocol::Udp, true);
    tmpl.supports_rcon = true;
    let mut server = sample_server(Some(25500));
    server.rcon_port = Some(25700);
    server.rcon_password = Some("secret-console".to_string());

    let spec = dummy_service.build_spec(&server, &tmpl, &HashMap::new(), &sample_config());

    assert_eq!(
        spec.env.get("ADMIN_PASSWORD").map(|s| s.as_str()),
        Some("secret-console")
    );
    assert_eq!(spec.env.get("RCON_PORT").map(|s| s.as_str()), Some("25575"));
    assert!(!spec.env.contains_key("ENABLE_RCON"));
    assert!(!spec.env.contains_key("RCON_ENABLED"));
    assert!(!spec.env.contains_key("RCON_PASSWORD"));
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
        rules: None,
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
    ) -> Result<Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>, DomainError>
    {
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
    async fn set_ip_reveal_at(&self, _: Uuid, _: Option<DateTime<Utc>>) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_scheduled_due_to_start(&self) -> Result<Vec<GameServer>, DomainError> {
        Ok(vec![])
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
    async fn archive_volume(
        &self,
        _volume: &str,
        _nom_fichier: &str,
    ) -> Result<VolumeArchive, DomainError> {
        unimplemented!("archivage non couvert par ce double de test")
    }
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
        Ok(Some(ContainerStatus {
            container_id: "cid".into(),
            state: ContainerState::Running,
            exit_code: None,
            error: None,
        }))
    }
    async fn list_managed_containers(&self) -> Result<Vec<ManagedContainer>, DomainError> {
        Ok(vec![])
    }
    async fn stats(&self, _: &str) -> Result<ContainerStats, DomainError> {
        Ok(ContainerStats::default())
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

// ── Forme des commandes RCON (point N4) ────────────────────────────────
//
// Ces tests ne disent RIEN de ce qu'un administrateur a le droit d'executer :
// il n'y a pas de liste blanche, et c'est un choix de produit. Ils verrouillent
// seulement la forme de ce qui peut partir vers le serveur de jeu.
mod commande_rcon {
    use super::super::valider_commande_rcon;

    #[test]
    fn accepte_une_commande_ordinaire() {
        assert!(valider_commande_rcon("say bonjour").is_ok());
        assert!(valider_commande_rcon("  op MonPseudo  ").is_ok());
    }

    #[test]
    fn refuse_une_commande_vide() {
        assert!(valider_commande_rcon("").is_err());
        assert!(valider_commande_rcon("   ").is_err());
    }

    #[test]
    fn refuse_les_caracteres_de_controle() {
        // Le point du garde-fou : selon l'implementation du serveur de jeu, un
        // saut de ligne peut etre lu comme un separateur de commandes.
        assert!(valider_commande_rcon("say bonjour\nstop").is_err());
        assert!(valider_commande_rcon("say bonjour\r\nban tout-le-monde").is_err());
        assert!(valider_commande_rcon("say bonjour\0stop").is_err());
    }

    #[test]
    fn refuse_au_dela_de_la_borne() {
        assert!(valider_commande_rcon(&"a".repeat(2_000)).is_ok());
        assert!(valider_commande_rcon(&"a".repeat(2_001)).is_err());
    }
}

// ── Reconnaissance des erreurs Docker qui justifient une recreation ──

#[test]
fn un_conteneur_absent_est_reconnu_comme_tel() {
    // Message reel remonte par docker-agent : la ligne garde un container_id
    // supprime en dehors de l'application (`docker rm`, `prune`, recreation
    // interrompue). Sans cette reconnaissance, le serveur restait bloque en
    // erreur alors qu'il suffisait de rebatir le conteneur.
    let erreur = DomainError::Internal(
        "docker-agent 502 Bad Gateway: {\"error\":\"Erreur interne : inspect container \
         ownership: Docker responded with status code 404: No such container: cdb8f005\"}"
            .into(),
    );
    assert!(super::is_missing_container_error(&erreur));
    assert!(!super::is_missing_network_error(&erreur));
}

#[test]
fn un_reseau_disparu_reste_distingue_du_conteneur_absent() {
    let erreur = DomainError::Internal("network abc123 not found".into());
    assert!(super::is_missing_network_error(&erreur));
    assert!(!super::is_missing_container_error(&erreur));
}

#[test]
fn une_erreur_ordinaire_ne_declenche_aucune_recreation() {
    // Rebatir un conteneur sur n'importe quelle erreur masquerait la cause
    // reelle — un port pris, une image absente — et la ferait revenir en
    // boucle.
    let erreur = DomainError::Internal("port 25565 already allocated".into());
    assert!(!super::is_missing_container_error(&erreur));
    assert!(!super::is_missing_network_error(&erreur));
}

// ── Tests de validation et provisioning (validate_create) ─────────────────

struct MockTemplateRepoWithTemplate {
    template: GameTemplate,
}
#[async_trait::async_trait]
impl GameTemplateRepository for MockTemplateRepoWithTemplate {
    async fn find_by_id(&self, _: Uuid) -> Result<Option<GameTemplate>, DomainError> {
        Ok(Some(self.template.clone()))
    }
    async fn find_by_slug(&self, _: &str) -> Result<Option<GameTemplate>, DomainError> {
        Ok(Some(self.template.clone()))
    }
    async fn list(&self) -> Result<Vec<GameTemplate>, DomainError> {
        Ok(vec![self.template.clone()])
    }
}

struct NonOperationalRuntime;
#[async_trait::async_trait]
impl ContainerRuntime for NonOperationalRuntime {
    async fn archive_volume(
        &self,
        _volume: &str,
        _nom_fichier: &str,
    ) -> Result<VolumeArchive, DomainError> {
        unimplemented!("archivage non couvert par ce double de test")
    }
    fn is_operational(&self) -> bool {
        false
    }
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

struct QuotaExceededServerRepo {
    count: i32,
    mem: i32,
}
#[async_trait::async_trait]
impl GameServerRepository for QuotaExceededServerRepo {
    async fn count_active_for_guild(&self, _: &str) -> Result<(i32, i32), DomainError> {
        Ok((self.count, self.mem))
    }
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
    async fn template_usages(
        &self,
        _: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, TemplateUsage>, DomainError> {
        Ok(std::collections::HashMap::new())
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
    async fn list_ip_reveal_due(&self) -> Result<Vec<GameServer>, DomainError> {
        Ok(vec![])
    }
    async fn list_awaiting_reveal_no_ping_today(&self) -> Result<Vec<GameServer>, DomainError> {
        Ok(vec![])
    }
    async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update_resources(&self, _: Uuid, _: i32, _: Option<f64>) -> Result<(), DomainError> {
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
    ) -> Result<Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>, DomainError>
    {
        Ok(vec![])
    }
    async fn purge_history(&self, _: i32) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn record_perf_sample(
        &self,
        _: Uuid,
        _: Option<i32>,
        _: Option<i64>,
        _: Option<i64>,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_config_dirty(&self, _: Uuid, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_closes_at(&self, _: Uuid, _: Option<DateTime<Utc>>) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_ip_reveal_at(&self, _: Uuid, _: Option<DateTime<Utc>>) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_scheduled_due_to_start(&self) -> Result<Vec<GameServer>, DomainError> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn test_validate_create_portal_disabled() {
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
    let mut cfg = sample_config();
    cfg.enabled = false;
    let cmd = sample_command();
    let res = dummy_service.validate_create(&cmd, &cfg).await;
    assert!(matches!(res, Err(DomainError::Forbidden(_))));
}

#[tokio::test]
async fn test_validate_create_invalid_name() {
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
    let cfg = sample_config();
    let mut cmd = sample_command();
    cmd.name = "".to_string();
    let res = dummy_service.validate_create(&cmd, &cfg).await;
    assert!(matches!(res, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn test_validate_create_template_not_allowed() {
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
    let cfg = sample_config();
    let mut cmd = sample_command();
    cmd.template_slug = "invalid-slug-not-allowed".to_string();
    let res = dummy_service.validate_create(&cmd, &cfg).await;
    assert!(matches!(res, Err(DomainError::Forbidden(_))));
}

#[tokio::test]
async fn test_validate_create_runtime_not_operational() {
    let tmpl = sample_template("minecraft-vanilla", TemplatePortProtocol::Tcp, true);
    let dummy_service = ManageGameServersService {
        server_repo: Arc::new(DummyServerRepo),
        template_repo: Arc::new(MockTemplateRepoWithTemplate { template: tmpl }),
        config_repo: Arc::new(DummyConfigRepo),
        audit_repo: Arc::new(DummyAuditRepo),
        port_allocator: Arc::new(DummyPortAllocator),
        container_runtime: Arc::new(NonOperationalRuntime),
        rcon_client: Arc::new(DummyRconClient),
        bot_config: Arc::new(DummyBotConfig),
    };
    let cfg = sample_config();
    let cmd = sample_command();
    let res = dummy_service.validate_create(&cmd, &cfg).await;
    assert!(matches!(res, Err(DomainError::NotImplemented(_))));
}

#[tokio::test]
async fn test_validate_create_memory_out_of_bounds() {
    let mut tmpl = sample_template("minecraft-vanilla", TemplatePortProtocol::Tcp, true);
    tmpl.min_memory_mb = 1024;
    tmpl.max_memory_mb = 4096;
    let dummy_service = ManageGameServersService {
        server_repo: Arc::new(DummyServerRepo),
        template_repo: Arc::new(MockTemplateRepoWithTemplate { template: tmpl }),
        config_repo: Arc::new(DummyConfigRepo),
        audit_repo: Arc::new(DummyAuditRepo),
        port_allocator: Arc::new(DummyPortAllocator),
        container_runtime: Arc::new(DummyContainerRuntime),
        rcon_client: Arc::new(DummyRconClient),
        bot_config: Arc::new(DummyBotConfig),
    };
    let cfg = sample_config();
    let mut cmd = sample_command();
    cmd.allocated_memory_mb = Some(8192); // Exceeds max
    let res = dummy_service.validate_create(&cmd, &cfg).await;
    assert!(matches!(res, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn test_validate_create_quota_server_limit_exceeded() {
    let tmpl = sample_template("minecraft-vanilla", TemplatePortProtocol::Tcp, true);
    let dummy_service = ManageGameServersService {
        server_repo: Arc::new(QuotaExceededServerRepo {
            count: 5,
            mem: 2048,
        }),
        template_repo: Arc::new(MockTemplateRepoWithTemplate { template: tmpl }),
        config_repo: Arc::new(DummyConfigRepo),
        audit_repo: Arc::new(DummyAuditRepo),
        port_allocator: Arc::new(DummyPortAllocator),
        container_runtime: Arc::new(DummyContainerRuntime),
        rcon_client: Arc::new(DummyRconClient),
        bot_config: Arc::new(DummyBotConfig),
    };
    let cfg = sample_config();
    let cmd = sample_command();
    let res = dummy_service.validate_create(&cmd, &cfg).await;
    assert!(matches!(res, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn test_validate_create_success() {
    let tmpl = sample_template("minecraft-vanilla", TemplatePortProtocol::Tcp, true);
    let dummy_service = ManageGameServersService {
        server_repo: Arc::new(QuotaExceededServerRepo {
            count: 1,
            mem: 2048,
        }),
        template_repo: Arc::new(MockTemplateRepoWithTemplate { template: tmpl }),
        config_repo: Arc::new(DummyConfigRepo),
        audit_repo: Arc::new(DummyAuditRepo),
        port_allocator: Arc::new(DummyPortAllocator),
        container_runtime: Arc::new(DummyContainerRuntime),
        rcon_client: Arc::new(DummyRconClient),
        bot_config: Arc::new(DummyBotConfig),
    };
    let cfg = sample_config();
    let cmd = sample_command();
    let res = dummy_service.validate_create(&cmd, &cfg).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_provisioning_render_env_and_release_ports() {
    let mut tmpl = sample_template("valheim", TemplatePortProtocol::Udp, true);
    tmpl.default_env = serde_json::json!({ "FOO": "bar", "NUM": 123 });
    let mut overrides = HashMap::new();
    overrides.insert("CUSTOM".to_string(), "val".to_string());
    overrides.insert("FOO".to_string(), "overridden".to_string());

    let env = ManageGameServersService::render_env(&tmpl, &overrides);
    assert_eq!(env.get("FOO").map(|s| s.as_str()), Some("overridden"));
    assert_eq!(env.get("CUSTOM").map(|s| s.as_str()), Some("val"));
    assert_eq!(env.get("NUM").map(|s| s.as_str()), Some("123"));

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
    dummy_service
        .release_ports(&[(PortKind::Game, 25565), (PortKind::Rcon, 25575)])
        .await;
}

#[tokio::test]
async fn test_provisioning_fail_start_cleanup_and_audit() {
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
    let err = DomainError::Internal("test failure".into());
    let res = dummy_service
        .fail_start_cleanup(
            Uuid::new_v4(),
            &[(PortKind::Game, 25565)],
            Some("vol_test"),
            "create_container",
            &err,
        )
        .await;
    assert!(res.is_ok());

    dummy_service
        .audit(
            "guild_1",
            Some(Uuid::new_v4()),
            Some("admin"),
            GameAuditAction::Start,
            serde_json::json!({}),
        )
        .await;
}

#[tokio::test]
async fn test_provisioning_upload_init_files_and_recreate_container() {
    let mut tmpl = sample_template("minecraft-vanilla", TemplatePortProtocol::Tcp, true);
    tmpl.init_files = vec![crate::nexus::domain::entities::game::template::InitFile {
        path: "/data/config.txt".into(),
        content: "server={{DEFAULT_VAR}}".into(),
    }];

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
    let s_id = Uuid::new_v4();
    let res = dummy_service.upload_init_files(s_id, "cid123", &tmpl).await;
    assert!(res.is_ok());

    let mut server = sample_server(Some(25500));
    server.container_id = Some("old_cid".into());
    server.volume_name = Some("vol_123".into());
    let cfg = sample_config();

    let cid = dummy_service
        .recreate_container(s_id, &server, &tmpl, &cfg)
        .await
        .unwrap();
    assert_eq!(cid, "cid");
}

#[tokio::test]
async fn test_render_template_edge_cases() {
    let mut env = HashMap::new();
    env.insert("FOO".into(), "bar".into());
    let res = super::super::manage_game_servers_service::provisioning::render_template(
        "Hello {{ FOO }} and {{ UNCLOSED",
        &env,
    );
    assert_eq!(res, "Hello bar and {{ UNCLOSED");
}

#[tokio::test]
async fn test_manage_game_servers_lifecycle_methods() {
    struct LifecycleServerRepo {
        server: Mutex<GameServer>,
    }
    #[async_trait::async_trait]
    impl GameServerRepository for LifecycleServerRepo {
        async fn create(&self, _: NewGameServer) -> Result<GameServer, DomainError> {
            Ok(self.server.lock().unwrap().clone())
        }
        async fn find_by_id(&self, _: Uuid) -> Result<Option<GameServer>, DomainError> {
            Ok(Some(self.server.lock().unwrap().clone()))
        }
        async fn list_by_guild(&self, _: &str) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![self.server.lock().unwrap().clone()])
        }
        async fn list_running(&self) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![self.server.lock().unwrap().clone()])
        }
        async fn list_active(&self) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![self.server.lock().unwrap().clone()])
        }
        async fn update_runtime(
            &self,
            _: Uuid,
            u: GameServerRuntimeUpdate,
        ) -> Result<(), DomainError> {
            let mut s = self.server.lock().unwrap();
            if let Some(st) = u.status {
                s.status = st;
            }
            if let Some(cid) = u.container_id {
                s.container_id = Some(cid);
            }
            if let Some(hp) = u.host_port {
                s.host_port = Some(hp);
            }
            if let Some(rp) = u.rcon_port {
                s.rcon_port = Some(rp);
            }
            Ok(())
        }
        async fn update_status(
            &self,
            _: Uuid,
            s: GameServerStatus,
            _: Option<&str>,
        ) -> Result<(), DomainError> {
            self.server.lock().unwrap().status = s;
            Ok(())
        }
        async fn try_transition_status(
            &self,
            _: Uuid,
            _: &[GameServerStatus],
            to: GameServerStatus,
        ) -> Result<bool, DomainError> {
            self.server.lock().unwrap().status = to;
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
            Ok((1, 1024))
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
            self.server.lock().unwrap().ip_revealed = true;
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

    struct MockRcon;
    #[async_trait::async_trait]
    impl RconClient for MockRcon {
        async fn execute(
            &self,
            _: &RconConnectionParams,
            cmd: &str,
        ) -> Result<crate::nexus::ports::outbound::game::rcon_client::RconResponse, DomainError>
        {
            Ok(
                crate::nexus::ports::outbound::game::rcon_client::RconResponse {
                    raw: format!("OK: {cmd}"),
                },
            )
        }
    }

    struct LifecycleBotConfig;
    #[async_trait::async_trait]
    impl BotConfigRepository for LifecycleBotConfig {
        async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
            Ok(vec![])
        }
        async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
            Ok(vec![
                BotGuildConfig {
                    id: Uuid::new_v4(),
                    guild_id: "guild_1".into(),
                    bot_name: "game-portal".into(),
                    config_key: "session_public_host".into(),
                    config_value: "play.example.com".into(),
                    updated_at: chrono::Utc::now(),
                },
                BotGuildConfig {
                    id: Uuid::new_v4(),
                    guild_id: "guild_1".into(),
                    bot_name: "game-portal".into(),
                    config_key: "enabled".into(),
                    config_value: "true".into(),
                    updated_at: chrono::Utc::now(),
                },
                BotGuildConfig {
                    id: Uuid::new_v4(),
                    guild_id: "guild_1".into(),
                    bot_name: "game-portal".into(),
                    config_key: "allowed_templates".into(),
                    config_value: "minecraft-vanilla".into(),
                    updated_at: chrono::Utc::now(),
                },
            ])
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

    let mut server = sample_server(Some(25500));
    server.status = GameServerStatus::Stopped;
    server.rcon_port = Some(25575);
    server.rcon_password = Some("pwd".into());

    let tmpl = sample_template("minecraft-vanilla", TemplatePortProtocol::Tcp, true);

    let service = ManageGameServersService {
        server_repo: Arc::new(LifecycleServerRepo {
            server: Mutex::new(server.clone()),
        }),
        template_repo: Arc::new(MockTemplateRepoWithTemplate { template: tmpl }),
        config_repo: Arc::new(DummyConfigRepo),
        audit_repo: Arc::new(DummyAuditRepo),
        port_allocator: Arc::new(DummyPortAllocator),
        container_runtime: Arc::new(DummyContainerRuntime),
        rcon_client: Arc::new(MockRcon),
        bot_config: Arc::new(LifecycleBotConfig),
    };

    // get
    let detail = service.get(server.id).await.unwrap();
    assert_eq!(detail.server.id, server.id);

    // list_for_guild
    let list = service.list_for_guild(&server.guild_id).await.unwrap();
    assert_eq!(list.len(), 1);

    // start
    let s_res = service.start(server.id, "actor_1").await;
    assert!(s_res.is_ok());

    // reveal_ip
    let rev = service.reveal_ip(server.id, "actor_1").await;
    assert!(rev.is_ok());

    // logs
    let logs = service.get_logs(server.id, 50).await.unwrap();
    assert!(logs.is_empty());

    // stats
    let st = service.get_stats(server.id).await.unwrap();
    assert_eq!(st.cpu_percent, 0.0);

    // execute_rcon
    let rcon_resp = service
        .execute_rcon(server.id, "say Hello", "actor_1")
        .await
        .unwrap();
    assert_eq!(rcon_resp, "OK: say Hello");

    // set_resources
    let res_change = service
        .update_resources(server.id, 2048, Some(2.0), "actor_1")
        .await;
    assert!(res_change.is_ok());

    // restart
    let restarted = service.restart(server.id, "actor_1").await;
    assert!(restarted.is_ok());

    // stop
    let stopped = service.stop(server.id, "actor_1").await;
    assert!(stopped.is_ok());

    // delete
    let deleted = service.delete(server.id, "actor_1").await;
    assert!(deleted.is_ok());
}

#[tokio::test]
async fn test_upload_init_files_failure() {
    struct FailingUploadRuntime;
    #[async_trait::async_trait]
    impl ContainerRuntime for FailingUploadRuntime {
        async fn archive_volume(
            &self,
            _volume: &str,
            _nom_fichier: &str,
        ) -> Result<VolumeArchive, DomainError> {
            unimplemented!("archivage non couvert par ce double de test")
        }
        fn is_operational(&self) -> bool {
            true
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
            Ok("cid".into())
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
            Err(DomainError::Internal("upload failed".into()))
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

    let mut tmpl = sample_template("minecraft-vanilla", TemplatePortProtocol::Tcp, true);
    tmpl.init_files = vec![crate::nexus::domain::entities::game::template::InitFile {
        path: "/data/config.txt".into(),
        content: "server=1".into(),
    }];

    let service = ManageGameServersService {
        server_repo: Arc::new(DummyServerRepo),
        template_repo: Arc::new(DummyTemplateRepo),
        config_repo: Arc::new(DummyConfigRepo),
        audit_repo: Arc::new(DummyAuditRepo),
        port_allocator: Arc::new(DummyPortAllocator),
        container_runtime: Arc::new(FailingUploadRuntime),
        rcon_client: Arc::new(DummyRconClient),
        bot_config: Arc::new(DummyBotConfig),
    };

    let res = service
        .upload_init_files(Uuid::new_v4(), "cid", &tmpl)
        .await;
    assert!(res.is_err());
}
