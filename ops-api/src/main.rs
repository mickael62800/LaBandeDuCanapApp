//! Point d'entree de l'API d'exploitation.

use std::sync::Arc;

use ops_api::{router, AppConfig, AppState};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Le recorder doit etre installe AVANT le routeur : une metrique emise
    // avant lui est perdue.
    platform_common_api::metrics::init_prometheus();

    let config = match AppConfig::from_env() {
        Ok(config) => Arc::new(config),
        Err(error) => {
            tracing::error!(%error, "configuration invalide");
            std::process::exit(1);
        }
    };

    // `connect_lazy` : l'API demarre meme si Postgres n'est pas encore pret et
    // se connecte a la premiere requete. Le healthcheck reste vert pendant le
    // demarrage de la base, ce qui evite un cycle de redemarrages.
    let pool = match sqlx::PgPool::connect_lazy(&config.database_url) {
        Ok(pool) => pool,
        Err(error) => {
            tracing::error!(%error, "URL de base invalide");
            std::process::exit(1);
        }
    };

    // ── Adaptateurs sortants ──
    let alert_rules_repo = Arc::new(
        ops_api::adapters::alert_rule_repository::PgAlertRuleRepository::new(pool.clone()),
    );
    let server_events: Arc<dyn ops_core::ports::outbound::server_event_repository::ServerEventRepository> =
        Arc::new(ops_api::adapters::server_event_repository::PgServerEventRepository::new(pool.clone()));
    // Docker passe par l'agent : ce processus ne monte jamais le socket.
    let docker_host: Arc<dyn ops_core::ports::outbound::docker_host::DockerHost> = Arc::new(
        ops_api::adapters::http_docker_host::HttpDockerHost::new(
            config.docker_agent_url.clone(),
            config.docker_agent_token.clone(),
        ),
    );

    // ── Cas d'usage ──
    let alert_rules_uc = Arc::new(
        ops_core::application::manage_alert_rules_service::ManageAlertRulesService::new(
            alert_rules_repo,
        ),
    );

    // ── Surveillance de fond ──
    // Demarree avant le serveur : le premier relevé sert de reference, et
    // l'endpoint qui la sert repond « pas encore de relevé » entre-temps.
    let container_monitor =
        ops_api::container_monitor::spawn(docker_host.clone(), server_events.clone());

    let redis_client = redis::Client::open(config.redis_url.as_str())
        .expect("Redis est requis pour l'API ops (deduplication d'alertes et logs)");
    
    ops_api::alerts_dispatcher::spawn(
        pool.clone(),
        redis_client.clone(),
        Some(container_monitor.clone()),
    );

    let log_repo = Arc::new(ops_api::adapters::log_repository::PgLogRepository::new(pool.clone()));
    let system_logs_uc: Arc<dyn ops_core::ports::inbound::manage_system_logs::ManageSystemLogsUseCase> =
        Arc::new(ops_core::application::manage_system_logs_service::ManageSystemLogsService::new(
            log_repo,
        ));

    // ── Securite de l'hote ──
    let ip_ban_repo = Arc::new(
        ops_api::adapters::ip_ban_repository::PgIpBanRepository::new(pool.clone()),
    );
    let host_ban_queue = Arc::new(ops_api::adapters::host_security::ban_queue::FileBanQueue::new());
    let fail2ban_reader = Arc::new(ops_api::adapters::host_security::fail2ban::Fail2banFileReader::new());
    let ip_bans_uc: Arc<dyn ops_core::ports::inbound::manage_ip_bans::ManageIpBansUseCase> =
        Arc::new(ops_core::application::manage_ip_bans_service::ManageIpBansService::new(
            ip_ban_repo,
            host_ban_queue,
            fail2ban_reader,
        ));

    let host_probe_reader = Arc::new(
        ops_api::adapters::host_security::probe_reader::FileHostProbeReader::new(),
    );
    let host_probe_uc: Arc<dyn ops_core::ports::inbound::read_host_probe::ReadHostProbeUseCase> =
        Arc::new(ops_core::application::read_host_probe_service::ReadHostProbeService::new(
            host_probe_reader,
        ));

    let security_log_repo = Arc::new(
        ops_api::adapters::security_log_repository::PgSecurityLogRepository::new(pool.clone()),
    );
    let security_logs_uc: Arc<dyn ops_core::ports::inbound::read_security_logs::ReadSecurityLogsUseCase> =
        Arc::new(ops_core::application::read_security_logs_service::ReadSecurityLogsService::new(
            security_log_repo,
        ));

    let security_audit_repo = Arc::new(
        ops_api::adapters::security_audit_repository::PgSecurityAuditRepository::new(pool.clone()),
    );
    let security_audit_uc: Arc<dyn ops_core::ports::inbound::manage_security_audit::ManageSecurityAuditUseCase> =
        Arc::new(ops_core::application::manage_security_audit_service::ManageSecurityAuditService::new(
            security_audit_repo,
        ));

    let tls_cert_uc: Arc<dyn ops_core::ports::inbound::read_tls_cert::ReadTlsCertUseCase> =
        Arc::new(ops_core::application::read_tls_cert_service::ReadTlsCertService::new(
            Arc::new(ops_api::adapters::host_security::tls_cert::FileTlsCertReader::new()),
        ));

    let geoip_uc: Arc<dyn ops_core::ports::inbound::lookup_geoip::LookupGeoIpUseCase> =
        Arc::new(ops_core::application::lookup_geoip_service::LookupGeoIpService::new(
            Arc::new(ops_api::adapters::geoip::IpApiGeoIpLookup::new()),
        ));

    let server_events_uc: Arc<dyn ops_core::ports::inbound::manage_server_events::ManageServerEventsUseCase> =
        Arc::new(ops_core::application::manage_server_events_service::ManageServerEventsService::new(
            server_events.clone(),
        ));

    let bind = config.bind_addr;
    let state = AppState {
        config,
        alert_rules_uc,
        docker_host,
        server_events,
        container_monitor,
        security_logs_uc,
        security_audit_uc,
        host_probe_uc,
        tls_cert_uc,
        ip_bans_uc,
        geoip_uc,
        server_events_uc,
        system_logs_uc,
        redis_client,
    };

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .expect("bind impossible");
    tracing::info!(%bind, "ops-api demarre");
    axum::serve(listener, router(state).into_make_service_with_connect_info::<std::net::SocketAddr>())
        .await
        .expect("serveur arrete");
}
