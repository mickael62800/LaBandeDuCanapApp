//! Port outbound : lecture des sondes de securite host (JSON cron).

use async_trait::async_trait;

use crate::ops::domain::entities::host_probe::HostProbe;
use crate::ops::domain::errors::DomainError;

#[async_trait]
pub trait HostProbeReader: Send + Sync {
    /// Lit le JSON brut de la sonde. `NotFound` si la sonde n'est pas
    /// installee (fichier absent), `Internal` si le JSON est illisible.
    async fn read(&self, probe: HostProbe) -> Result<serde_json::Value, DomainError>;
}
