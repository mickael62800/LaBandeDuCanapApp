//! Sondes de securite host : donnees collectees par des cron host et exposees
//! en JSON sous `/var/lib/sentinel/`. L'API ne fait que relayer ces blobs vers
//! le web (pass-through), d'ou un type traversant `serde_json::Value` plutot
//! qu'une entite par sonde. L'enum identifie la sonde de maniere agnostique du
//! chemin de fichier (detail infra resolu par l'adapter).

/// Identifie une sonde de securite host (lecture seule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostProbe {
    SshFailures,
    DiskTrend,
    Connections,
    OpenPorts,
    Trivy,
    TlsErrors,
    FileIntegrity,
    Outbound,
    NginxSuspicious,
}

impl HostProbe {
    /// Nom public de la sonde (utilise dans les messages de setup).
    pub fn feature(&self) -> &'static str {
        match self {
            HostProbe::SshFailures => "ssh-failures",
            HostProbe::DiskTrend => "disk-trend",
            HostProbe::Connections => "connections",
            HostProbe::OpenPorts => "open-ports",
            HostProbe::Trivy => "trivy",
            HostProbe::TlsErrors => "tls-errors",
            HostProbe::FileIntegrity => "file-integrity",
            HostProbe::Outbound => "outbound",
            HostProbe::NginxSuspicious => "nginx-suspicious",
        }
    }
}

#[cfg(test)]
#[path = "tests/host_probe.rs"]
mod tests;
