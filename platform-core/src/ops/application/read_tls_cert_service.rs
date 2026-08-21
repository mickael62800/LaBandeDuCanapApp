//! Implementation du use case cert TLS. Pass-through vers le port outbound
//! (la lecture fichier / openssl / parse x509 est dans l'adapter).

use std::sync::Arc;

use async_trait::async_trait;

use crate::ops::domain::entities::tls_cert::TlsCertInfo;
use crate::ops::domain::errors::DomainError;
use crate::ops::ports::inbound::read_tls_cert::ReadTlsCertUseCase;
use crate::ops::ports::outbound::tls_cert_reader::TlsCertReader;

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

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeTlsCertReader;
    #[async_trait]
    impl TlsCertReader for FakeTlsCertReader {
        async fn read(&self) -> Result<TlsCertInfo, DomainError> {
            Ok(TlsCertInfo {
                domain: "example.com".into(),
                issuer: "CA".into(),
                subject: "CN=example.com".into(),
                not_before: "2024-01-01".into(),
                not_after: "2025-01-01".into(),
                days_until_expiry: 365,
                is_expired: false,
                is_warning: false,
            })
        }
    }

    #[test]
    fn service_can_be_created() {
        let _service = ReadTlsCertService::new(Arc::new(FakeTlsCertReader));
    }

    #[tokio::test]
    async fn read_delegates_to_reader() {
        let service = ReadTlsCertService::new(Arc::new(FakeTlsCertReader));
        let result = service.read().await;
        assert!(result.is_ok());
    }
}
