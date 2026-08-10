//! Implementation du use case sondes host. Pass-through vers le port outbound
//! (aucune logique : les sondes sont des blobs host opaques).

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::host_probe::HostProbe;
use crate::domain::errors::DomainError;
use crate::ports::inbound::read_host_probe::ReadHostProbeUseCase;
use crate::ports::outbound::host_probe_reader::HostProbeReader;

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
