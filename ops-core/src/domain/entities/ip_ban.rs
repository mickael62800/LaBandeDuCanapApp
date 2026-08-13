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

// `BanIpOutcome` a ete supprime avec son unique champ, `deleted_logs`.
//
// Bannir une IP ne purge plus ses logs — la mesure detruisait les preuves qui
// la justifiaient. Le champ a survecu au comportement qu'il decrivait : il
// valait toujours 0, et l'interface annoncait « 0 logs purges » a chaque ban.
// Un contrat qui rend compte d'une action qui n'a plus lieu est pire qu'un
// contrat absent : il se lit comme une mesure devenue inefficace.
//
// `ban` renvoie donc `()`. La retention des logs est le travail de la purge
// programmee (`/security/cleanup`), explicite et auditee.

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
