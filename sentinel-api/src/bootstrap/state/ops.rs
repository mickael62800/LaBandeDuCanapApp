//! Etat du domaine ops : la MACHINE HOTE, pas Discord.
//!
//! Conteneurs Docker, disques et sondes systeme, logs techniques des services,
//! securite de l'hote (certificat TLS, IP bannies, journal d'administration),
//! regles d'alerte.
//!
//! POURQUOI IL EST DISTINCT DE `SystemState`
//!
//! `system` melangeait deux choses de nature differente : le metier Discord de
//! la plateforme (tickets, OAuth, reset de guilde, lockdown, exports) et
//! l'exploitation de la machine qui l'heberge. Or cette machine heberge aussi
//! Nexus et Atrium : ces ecrans ne sont pas « du Sentinel », ils sont
//! transverses. La barre laterale du back-office les a deja separes en un
//! univers « Exploitation » ; ce sous-etat en est la contrepartie cote API.
//!
//! Un handler qui declare `State<OpsState>` se voit interdire par le
//! compilateur de toucher aux tickets ou a l'OAuth — et reciproquement.

use std::sync::Arc;

use axum::extract::FromRef;
use ops_core::ports::inbound::lookup_geoip::LookupGeoIpUseCase;
use ops_core::ports::inbound::manage_ip_bans::ManageIpBansUseCase;
use ops_core::ports::inbound::manage_security_audit::ManageSecurityAuditUseCase;
use ops_core::ports::inbound::manage_server_events::ManageServerEventsUseCase;
use ops_core::ports::inbound::manage_system_logs::ManageSystemLogsUseCase;
use ops_core::ports::inbound::read_host_probe::ReadHostProbeUseCase;
use ops_core::ports::inbound::read_security_logs::ReadSecurityLogsUseCase;
use ops_core::ports::inbound::read_tls_cert::ReadTlsCertUseCase;
use ops_core::ports::outbound::docker_host::DockerHost;
use ops_core::ports::outbound::log_repository::LogRepository;
use ops_core::ports::outbound::service_registry::ServiceRegistry;
use ops_core::ports::outbound::system_probe::SystemProbe;

use crate::adapters::outbound::ws::broadcaster::EventBroadcaster;
use crate::bootstrap::state::AppState;

/// Ports de l'exploitation de la machine hote.
#[derive(Clone)]
pub struct OpsState {
    // ── Etat de la machine et des services ──
    /// Sondes sante (taille/disponibilite BDD). Les handlers health/info
    /// passent par ici, jamais par `pg_pool`.
    pub system_probe: Arc<dyn SystemProbe>,
    /// Daemon Docker de l'hote (listing, actions, prune, df).
    pub docker_host: Arc<dyn DockerHost>,
    /// Poll Docker chaque minute, detecte les changements d'etat.
    pub container_monitor: Option<
        Arc<tokio::sync::RwLock<crate::bootstrap::container_monitor::ContainerMonitorState>>,
    >,

    // ── Journaux techniques ──
    /// Decouverte des bots et workers en ligne. Consomme par le tableau de
    /// bord, qui compose metier et sante des services au niveau du handler.
    pub service_registry: Arc<dyn ServiceRegistry>,
    pub system_logs_uc: Arc<dyn ManageSystemLogsUseCase>,
    pub log_repo: Arc<dyn LogRepository>,
    pub server_events_uc: Arc<dyn ManageServerEventsUseCase>,

    // ── Securite de l'hote ──
    pub security_logs_uc: Arc<dyn ReadSecurityLogsUseCase>,
    pub security_audit_uc: Arc<dyn ManageSecurityAuditUseCase>,
    pub host_probe_uc: Arc<dyn ReadHostProbeUseCase>,
    pub tls_cert_uc: Arc<dyn ReadTlsCertUseCase>,
    pub ip_bans_uc: Arc<dyn ManageIpBansUseCase>,
    pub geoip_uc: Arc<dyn LookupGeoIpUseCase>,
    /// Suivi req/IP en memoire pour le ban automatique.
    pub rate_limiter: Option<Arc<crate::adapters::outbound::system::rate_limiter::RateLimiter>>,

    // ── Dependances transverses ──
    pub broadcaster: Arc<EventBroadcaster>,
    pub redis_client: redis::Client,
}

impl FromRef<AppState> for OpsState {
    fn from_ref(state: &AppState) -> Self {
        state.ops.clone()
    }
}
