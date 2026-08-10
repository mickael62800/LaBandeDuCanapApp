//! Adapter du port `HostProbeReader` : lit les fichiers JSON exposes par les
//! cron host sous `/var/lib/sentinel/`. Le mapping sonde -> chemin (detail
//! infra) vit ici.

use async_trait::async_trait;

use ops_core::domain::entities::host_probe::HostProbe;
use sentinel_core::domain::errors::DomainError;
use ops_core::ports::outbound::host_probe_reader::HostProbeReader;

#[derive(Default)]
pub struct FileHostProbeReader;

impl FileHostProbeReader {
    pub fn new() -> Self {
        Self
    }

    fn path(probe: HostProbe) -> &'static str {
        match probe {
            HostProbe::SshFailures => "/var/lib/sentinel/ssh-failures.json",
            HostProbe::DiskTrend => "/var/lib/sentinel/disk-trend.json",
            HostProbe::Connections => "/var/lib/sentinel/connections.json",
            HostProbe::OpenPorts => "/var/lib/sentinel/open-ports.json",
            HostProbe::Trivy => "/var/lib/sentinel/trivy.json",
            HostProbe::TlsErrors => "/var/lib/sentinel/tls-errors.json",
            HostProbe::FileIntegrity => "/var/lib/sentinel/file-integrity.json",
            HostProbe::Outbound => "/var/lib/sentinel/outbound.json",
            HostProbe::NginxSuspicious => "/var/lib/sentinel/nginx-suspicious.json",
        }
    }
}

#[async_trait]
impl HostProbeReader for FileHostProbeReader {
    async fn read(&self, probe: HostProbe) -> Result<serde_json::Value, DomainError> {
        let path = Self::path(probe);
        let feature = probe.feature();
        let raw = std::fs::read_to_string(path).map_err(|e| {
            DomainError::NotFound(format!(
                "{feature} non disponible. Setup : sudo bash infrastructure/scripts/setup-host-security.sh {feature}. (lecture {path}: {e})"
            ))
        })?;
        serde_json::from_str(&raw).map_err(|e| DomainError::Internal(format!("parse {path}: {e}")))
    }
}
