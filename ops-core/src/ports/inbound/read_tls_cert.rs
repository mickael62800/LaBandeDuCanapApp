//! Use case : info d'expiration du certificat TLS web.

use async_trait::async_trait;

use crate::domain::entities::tls_cert::TlsCertInfo;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ReadTlsCertUseCase: Send + Sync {
    async fn read(&self) -> Result<TlsCertInfo, DomainError>;
}
