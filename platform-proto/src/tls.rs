//! Helpers TLS pour le mTLS gRPC inter-services.
//!
//! Active uniquement quand `GRPC_TLS_DIR` est defini en env. Sinon le serveur
//! et les clients restent en plain HTTP/2 (mode dev / migration progressive).
//!
//! Conteneur :
//!   - serveur (api) : monte `grpc_certs:/grpc-certs:ro` + GRPC_TLS_DIR=/grpc-certs
//!   - client (workers/bot) : meme volume + meme env var
//!
//! Cert layout (genere par scripts/gen-grpc-certs.sh) :
//!   /grpc-certs/ca.pem
//!   /grpc-certs/server.pem + server.key
//!   /grpc-certs/client.pem + client.key

use std::path::Path;
use std::path::PathBuf;

use tonic::transport::Certificate;
use tonic::transport::ClientTlsConfig;
use tonic::transport::Identity;
use tonic::transport::ServerTlsConfig;

/// Lit `GRPC_TLS_DIR` depuis l'env. Retourne None si non defini -> mode plain.
pub fn tls_dir() -> Option<PathBuf> {
    std::env::var("GRPC_TLS_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

/// Construit la config TLS serveur (cert + key + verifie le client via CA).
/// Active mTLS : le client DOIT presenter un cert signe par notre CA.
pub fn server_tls_config(dir: &Path) -> Result<ServerTlsConfig, std::io::Error> {
    let server_pem = std::fs::read(dir.join("server.pem"))?;
    let server_key = std::fs::read(dir.join("server.key"))?;
    let ca_pem = std::fs::read(dir.join("ca.pem"))?;

    let identity = Identity::from_pem(&server_pem, &server_key);
    let ca = Certificate::from_pem(&ca_pem);

    Ok(ServerTlsConfig::new().identity(identity).client_ca_root(ca))
}

/// Construit la config TLS client (cert client + verifie le serveur via CA).
/// `domain` doit matcher le SAN du cert serveur (ex: "api").
pub fn client_tls_config(dir: &Path, domain: &str) -> Result<ClientTlsConfig, std::io::Error> {
    let client_pem = std::fs::read(dir.join("client.pem"))?;
    let client_key = std::fs::read(dir.join("client.key"))?;
    let ca_pem = std::fs::read(dir.join("ca.pem"))?;

    let identity = Identity::from_pem(&client_pem, &client_key);
    let ca = Certificate::from_pem(&ca_pem);

    Ok(ClientTlsConfig::new()
        .ca_certificate(ca)
        .identity(identity)
        .domain_name(domain))
}
