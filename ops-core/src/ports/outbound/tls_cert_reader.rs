//! Port outbound : recupere et parse le certificat TLS du domaine web.

use async_trait::async_trait;

use crate::domain::entities::tls_cert::TlsCertInfo;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait TlsCertReader: Send + Sync {
    /// Lit le cert du domaine web courant (config infra) et en extrait l'info
    /// d'expiration.
    async fn read(&self) -> Result<TlsCertInfo, DomainError>;
}
