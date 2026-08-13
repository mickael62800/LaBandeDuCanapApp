//! Use case : expose les sondes de securite host au panel. Pass-through du
//! JSON ; la selection se fait via l'enum domaine `HostProbe`.

use async_trait::async_trait;

use crate::ops::domain::entities::host_probe::HostProbe;
use crate::ops::domain::errors::DomainError;

#[async_trait]
pub trait ReadHostProbeUseCase: Send + Sync {
    async fn read(&self, probe: HostProbe) -> Result<serde_json::Value, DomainError>;
}
