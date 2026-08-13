//! Adapter du port `HostProbeReader` : lit les fichiers JSON exposes par les
//! cron host sous `/var/lib/sentinel/`. Le mapping sonde -> chemin (detail
//! infra) vit ici.

use async_trait::async_trait;

use platform_core::ops::domain::entities::host_probe::HostProbe;
use platform_core::ops::domain::errors::DomainError;
use platform_core::ops::ports::outbound::host_probe_reader::HostProbeReader;

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

        // Lecture bloquante deportee : ces fichiers sont ecrits par des cron
        // hote et `trivy.json` atteint plusieurs Mo. Un `read_to_string` direct
        // retenait un thread du runtime pendant l'I/O.
        let raw = tokio::fs::read_to_string(path).await.map_err(|e| {
            // Le chemin exact et l'erreur systeme restent dans les logs : ce
            // message-ci part au client tel quel (`public_message` ne masque
            // que les 5xx), et decrire l'arborescence de l'hote dans une 404
            // n'aide que celui qui la cartographie.
            tracing::warn!(error = %e, path, "sonde hote illisible");
            DomainError::NotFound(format!(
                "{feature} non disponible. Setup : sudo bash infrastructure/scripts/setup-host-security.sh {feature}"
            ))
        })?;

        serde_json::from_str(&raw).map_err(|e| {
            tracing::error!(error = %e, path, "sonde hote au format invalide");
            DomainError::Internal("sonde hote au format invalide".into())
        })
    }
}
