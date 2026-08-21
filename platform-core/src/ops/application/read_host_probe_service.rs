//! Implementation du use case sondes host. Pass-through vers le port outbound
//! (aucune logique : les sondes sont des blobs host opaques).

use std::sync::Arc;

use async_trait::async_trait;

use crate::ops::domain::entities::host_probe::HostProbe;
use crate::ops::domain::errors::DomainError;
use crate::ops::ports::inbound::read_host_probe::ReadHostProbeUseCase;
use crate::ops::ports::outbound::host_probe_reader::HostProbeReader;

pub struct ReadHostProbeService {
    reader: Arc<dyn HostProbeReader>,
}

impl ReadHostProbeService {
    pub fn new(reader: Arc<dyn HostProbeReader>) -> Self {
        Self { reader }
    }
}

#[async_trait]
impl ReadHostProbeUseCase for ReadHostProbeService {
    async fn read(&self, probe: HostProbe) -> Result<serde_json::Value, DomainError> {
        self.reader.read(probe).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeHostProbeReader;
    #[async_trait]
    impl HostProbeReader for FakeHostProbeReader {
        async fn read(&self, _probe: HostProbe) -> Result<serde_json::Value, DomainError> {
            Ok(serde_json::json!({"cpu": 50}))
        }
    }

    #[test]
    fn service_can_be_created() {
        let _service = ReadHostProbeService::new(Arc::new(FakeHostProbeReader));
    }

    #[tokio::test]
    async fn read_delegates_to_reader() {
        let service = ReadHostProbeService::new(Arc::new(FakeHostProbeReader));
        let probe = HostProbe::SshFailures;
        let result = service.read(probe).await;
        assert!(result.is_ok());
    }
}
