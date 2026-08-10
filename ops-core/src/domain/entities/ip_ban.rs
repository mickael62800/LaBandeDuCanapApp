//! Entites du domaine "bans IP" (panel securite).
//!
//! Deux sources de verite distinctes :
//!   - `manual_ip_bans` (DB) : bans declenches depuis le panel admin.
//!   - fail2ban (host) : bans automatiques, exposes en JSON par un cron host.
//! L'application effective des regles reseau passe par un fichier-shim lu par
//! un cron host (cf. port outbound `HostBanQueue`).

use chrono::{DateTime, Utc};

/// Un ban IP manuel actif (declenche via le panel, pas encore leve).
#[derive(Debug, Clone)]
pub struct ManualIpBan {
    pub ip: String,
    pub banned_at: DateTime<Utc>,
    pub banned_by: Option<String>,
    pub reason: Option<String>,
}

/// Resultat d'une demande de ban : ce que l'appelant doit reporter.
#[derive(Debug, Clone)]
pub struct BanIpOutcome {
    /// Nombre de logs API purges pour cette IP (best-effort).
    pub deleted_logs: u64,
}

/// Statut fail2ban tel qu'expose par le cron host (lecture seule).
#[derive(Debug, Clone)]
pub struct Fail2banStatus {
    pub updated_at: String,
    pub jails: Vec<Fail2banJail>,
}

/// Une "jail" fail2ban et ses IPs bannies.
#[derive(Debug, Clone)]
pub struct Fail2banJail {
    pub name: String,
    pub total_banned: i64,
    pub banned_ips: Vec<String>,
}

impl Fail2banStatus {
    /// Nombre total d'IPs bannies, toutes jails confondues.
    pub fn total_banned_ips(&self) -> usize {
        self.jails.iter().map(|j| j.banned_ips.len()).sum()
    }
}

#[cfg(test)]
#[path = "tests/ip_ban.rs"]
mod tests;
