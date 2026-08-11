//! Helpers de connection gRPC vers l'API Sentinel.
//!
//! Centralise le boilerplate `Endpoint::from_shared + connect_timeout +
//! connect + with_interceptor(Authorization Bearer)` duplique dans
//! `export-worker/drain_export_jobs`.
//!
//! Lit `GRPC_API_URL` (default `http://127.0.0.1:50051`) et `API_KEY`
//! depuis l'environnement.
//!
//! # Pourquoi ici et plus dans `platform-common-worker`
//!
//! Ce module s'appuie sur `sentinel_proto::tls` (certificats mTLS de la
//! plateforme Sentinel) et n'a jamais eu qu'un appelant :
//! `domains/export/drain_export_jobs.rs`. Le loger dans le socle des trois
//! workers y faisait entrer `sentinel-proto`, que `nexus-worker` et
//! `atrium-worker` compilaient sans jamais l'utiliser. Un crate socle se
//! definit par sa surface de dependances, pas par la commodite de rangement.

use std::time::Duration;

use tonic::transport::{Channel, Endpoint};

const DEFAULT_GRPC_URL: &str = "http://127.0.0.1:50051";
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 5;
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

/// Connecte un Channel gRPC vers `GRPC_API_URL` avec timeouts par defaut
/// (connect 5s, request 30s). Active mTLS si `GRPC_TLS_DIR` defini en env.
/// Retourne une `String` d'erreur prete a remonter au scheduler.
pub async fn connect() -> Result<Channel, String> {
    let url = std::env::var("GRPC_API_URL").unwrap_or_else(|_| DEFAULT_GRPC_URL.to_string());

    // Si mTLS active, force https:// dans l'URL. tonic exige https pour
    // declencher le handshake TLS lors du connect.
    let effective_url = if sentinel_proto::tls::tls_dir().is_some() {
        if let Some(rest) = url.strip_prefix("http://") {
            format!("https://{rest}")
        } else if !url.starts_with("https://") {
            format!("https://{url}")
        } else {
            url.clone()
        }
    } else {
        url.clone()
    };

    let endpoint = Endpoint::from_shared(effective_url.clone())
        .map_err(|e| format!("invalid GRPC_API_URL {url}: {e}"))?
        .connect_timeout(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS));

    // mTLS optionnel : active si GRPC_TLS_DIR defini.
    // tls_config(self, ...) consomme self -> on construit la chaine en
    // une seule expression.
    let endpoint = match sentinel_proto::tls::tls_dir() {
        Some(dir) => {
            let domain = url
                .strip_prefix("http://")
                .or_else(|| url.strip_prefix("https://"))
                .unwrap_or(&url)
                .split(':')
                .next()
                .unwrap_or("api");
            let tls = sentinel_proto::tls::client_tls_config(&dir, domain)
                .map_err(|e| format!("read TLS certs: {e}"))?;
            endpoint
                .tls_config(tls)
                .map_err(|e| format!("tls_config gRPC: {e}"))?
        }
        None => endpoint,
    };

    endpoint
        .connect()
        .await
        .map_err(|e| format!("connect gRPC {url}: {e}"))
}
