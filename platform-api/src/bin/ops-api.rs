//! Point d'entree de l'API d'exploitation.

use std::sync::Arc;

use platform_api::ops::{router, AppConfig, AppState};

#[tokio::main]
async fn main() {
    run().await;
}

pub async fn run() {
    let _ = dotenvy::dotenv();
    if std::env::var_os("PLATFORM_API_UNIFIED_RUNTIME").is_none() {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .init();
    }

    // Le recorder doit etre installe AVANT le routeur : une metrique emise
    // avant lui est perdue.
    platform_api::shared::metrics::init_prometheus();

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
        platform_api::ops::adapters::alert_rule_repository::PgAlertRuleRepository::new(
            pool.clone(),
        ),
    );
    let server_events: Arc<
        dyn platform_core::ops::ports::outbound::server_event_repository::ServerEventRepository,
    > = Arc::new(ops_adapters::server_event_repository::PgServerEventRepository::new(pool.clone()));
    // Docker passe par l'agent : ce processus ne monte jamais le socket.
    let docker_host: Arc<dyn platform_core::ops::ports::outbound::docker_host::DockerHost> =
        Arc::new(ops_adapters::http_docker_host::HttpDockerHost::new(
            config.docker_agent_url.clone(),
            config.docker_agent_token.clone(),
        ));

    // ── Cas d'usage ──
    let alert_rules_uc = Arc::new(
        platform_core::ops::application::manage_alert_rules_service::ManageAlertRulesService::new(
            alert_rules_repo,
        ),
    );

    let redis_client = {
        let client = redis::Client::open(config.redis_url.as_str())
            .expect("Redis est requis pour l'API ops (snapshots et logs)");
        // ConnectionManager : une seule connexion multiplexee, auto-reconnectante,
        // partagee (par clone) entre toutes les requetes. ops-api demarre apres
        // `redis` (service_healthy cote compose), donc la connexion initiale
        // reussit ; ensuite le manager retente tout seul en cas de coupure.
        redis::aio::ConnectionManager::new(client)
            .await
            .expect("connexion Redis (ConnectionManager) impossible")
    };

    let log_repo =
        Arc::new(platform_api::ops::adapters::log_repository::PgLogRepository::new(pool.clone()));
    let system_logs_uc: Arc<
        dyn platform_core::ops::ports::inbound::manage_system_logs::ManageSystemLogsUseCase,
    > = Arc::new(
        platform_core::ops::application::manage_system_logs_service::ManageSystemLogsService::new(
            log_repo,
        ),
    );

    // ── Securite de l'hote ──
    let ip_ban_repo = Arc::new(
        platform_api::ops::adapters::ip_ban_repository::PgIpBanRepository::new(pool.clone()),
    );
    let host_ban_queue =
        Arc::new(platform_api::ops::adapters::host_security::ban_queue::FileBanQueue::new());
    let fail2ban_reader =
        Arc::new(platform_api::ops::adapters::host_security::fail2ban::Fail2banFileReader::new());
    let ip_bans_uc: Arc<
        dyn platform_core::ops::ports::inbound::manage_ip_bans::ManageIpBansUseCase,
    > = Arc::new(
        platform_core::ops::application::manage_ip_bans_service::ManageIpBansService::new(
            ip_ban_repo,
            host_ban_queue,
            fail2ban_reader,
        ),
    );

    let host_probe_reader = Arc::new(
        platform_api::ops::adapters::host_security::probe_reader::FileHostProbeReader::new(),
    );
    let host_probe_uc: Arc<
        dyn platform_core::ops::ports::inbound::read_host_probe::ReadHostProbeUseCase,
    > = Arc::new(
        platform_core::ops::application::read_host_probe_service::ReadHostProbeService::new(
            host_probe_reader,
        ),
    );

    let security_log_repo = Arc::new(
        platform_api::ops::adapters::security_log_repository::PgSecurityLogRepository::new(
            pool.clone(),
        ),
    );
    let security_logs_uc: Arc<
        dyn platform_core::ops::ports::inbound::read_security_logs::ReadSecurityLogsUseCase,
    > = Arc::new(
        platform_core::ops::application::read_security_logs_service::ReadSecurityLogsService::new(
            security_log_repo,
        ),
    );

    let security_audit_repo = Arc::new(
        platform_api::ops::adapters::security_audit_repository::PgSecurityAuditRepository::new(
            pool.clone(),
            platform_api::ops::adapters::auth_logins::AuthLoginsClient::new(
                std::env::var("AUTH_API_URL").unwrap_or_else(|_| "http://auth-api:8096".into()),
                std::env::var("AUTH_API_TOKEN").unwrap_or_default(),
            ),
        ),
    );
    let security_audit_uc: Arc<
        dyn platform_core::ops::ports::inbound::manage_security_audit::ManageSecurityAuditUseCase,
    > = Arc::new(
        platform_core::ops::application::manage_security_audit_service::ManageSecurityAuditService::new(
            security_audit_repo,
        ),
    );

    let tls_cert_uc: Arc<
        dyn platform_core::ops::ports::inbound::read_tls_cert::ReadTlsCertUseCase,
    > = Arc::new(
        platform_core::ops::application::read_tls_cert_service::ReadTlsCertService::new(Arc::new(
            platform_api::ops::adapters::host_security::tls_cert::FileTlsCertReader::new(),
        )),
    );

    let geoip_uc: Arc<dyn platform_core::ops::ports::inbound::lookup_geoip::LookupGeoIpUseCase> =
        Arc::new(
            platform_core::ops::application::lookup_geoip_service::LookupGeoIpService::new(
                Arc::new(platform_api::ops::adapters::geoip::IpApiGeoIpLookup::new()),
            ),
        );

    let server_events_uc: Arc<
        dyn platform_core::ops::ports::inbound::manage_server_events::ManageServerEventsUseCase,
    > = Arc::new(
        platform_core::ops::application::manage_server_events_service::ManageServerEventsService::new(
            server_events.clone(),
        ),
    );

    let bind = config.bind_addr;
    let state = AppState {
        config,
        alert_rules_uc,
        docker_host,
        server_events,
        security_logs_uc,
        security_audit_uc,
        host_probe_uc,
        tls_cert_uc,
        ip_bans_uc,
        geoip_uc,
        server_events_uc,
        system_logs_uc,
        redis_client,
        pg_pool: pool.clone(),
    };

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .expect("bind impossible");
    tracing::info!(%bind, "ops-api demarre");
    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("serveur arrete");
}

async fn shutdown_signal() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.expect("ecoute Ctrl+C") };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("ecoute SIGTERM")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Ctrl+C recu"),
        _ = terminate => tracing::info!("SIGTERM recu"),
    }
}
