//! GET/DELETE /api/security/* — surveillance attaques et integrite serveur.
//!
//! Tous les endpoints sont gates admin+ (require_role).
//! Sources :
//!   - logs : table `logs` (alimentee par api_logger_middleware)
//!   - audit_logs : table `audit_logs` (Discord events + extension audit_docker)
//!   - cert TLS : lecture du fichier /etc/letsencrypt/live/{domain}/cert.pem
//!   - sondes host : fichiers JSON exposes par les cron de l'hote
//!
//! Decoupe par domaine : `logs` (agregations API), `bans` (fail2ban + bans
//! manuels), `probes` (sondes host JSON), `audit` (journal + logins + cleanup),
//! `tls` (certificat + erreurs handshake), `geoip`. Les helpers partages
//! (`read_probe`, `record_event`, `actor_from`) sont ici.

pub mod audit;
pub mod bans;
pub mod geoip;
pub mod logs;
pub mod probes;
pub mod tls;

use axum::http::HeaderMap;
use ops_core::domain::entities::host_probe::HostProbe;

use crate::{ApiError, AppState};

/// Lit une sonde host via le use case et la deserialise dans le DTO de reponse.
/// Toute l'infra (fichier, chemin) est dans l'adapter outbound.
pub(crate) async fn read_probe<T: for<'de> serde::Deserialize<'de>>(
    state: &AppState,
    probe: HostProbe,
) -> Result<T, ApiError> {
    let value = state.host_probe_uc.read(probe).await?;
    serde_json::from_value(value).map_err(|e| {
        ApiError(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("parse {}: {e}", probe.feature()),
        )
    })
}

/// Journalise une action d'exploitation, sans jamais la faire echouer.
///
/// Une action de securite qui a REUSSI ne doit pas remonter une erreur parce
/// que sa trace n'a pas pu s'ecrire : on prefere perdre la ligne de journal
/// que faire croire a l'operateur que le bannissement n'a pas eu lieu.
pub(crate) async fn record_event(
    repo: &std::sync::Arc<
        dyn ops_core::ports::outbound::server_event_repository::ServerEventRepository,
    >,
    actor: &str,
    actor_name: Option<&str>,
    action: &str,
    target: Option<&str>,
    severity: &str,
    details: serde_json::Value,
) {
    if let Err(error) = repo
        .record(actor, actor_name, action, target, severity, details)
        .await
    {
        tracing::warn!(%error, action, "journalisation d'un evenement impossible");
    }
}

/// Identifiant Discord de l'operateur, remonte par nginx (X-Actor-Id).
///
/// Sans cette remontee, l'audit des bannissements et des purges perdrait son
/// auteur : on saurait qu'une IP a ete bannie, jamais par qui.
pub(crate) fn actor_from(headers: &HeaderMap) -> String {
    headers
        .get("x-actor-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .unwrap_or("inconnu")
        .to_owned()
}
