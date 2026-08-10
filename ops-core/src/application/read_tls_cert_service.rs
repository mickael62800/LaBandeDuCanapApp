//! Implementation du use case cert TLS. Pass-through vers le port outbound
//! (la lecture fichier / openssl / parse x509 est dans l'adapter).

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::tls_cert::TlsCertInfo;
use crate::domain::errors::DomainError;
use crate::ports::inbound::read_tls_cert::ReadTlsCertUseCase;
use crate::ports::outbound::tls_cert_reader::TlsCertReader;

pub struct ReadTlsCertService {
    reader: Arc<dyn TlsCertReader>,
}

impl ReadTlsCertService {
    pub fn new(reader: Arc<dyn TlsCertReader>) -> Self {
        Self { reader }
    }
}

#[async_trait]
impl ReadTlsCertUseCase for ReadTlsCertService {
    async fn read(&self) -> Result<TlsCertInfo, DomainError> {
        self.reader.read().await
    }
}
