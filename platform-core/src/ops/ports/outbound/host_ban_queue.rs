//! Ports outbound vers l'host pour les bans IP :
//!   - `HostBanQueue` : file-shim (l'API ecrit une ligne, un cron host applique
//!     `ufw deny` / `ufw delete` puis vide le fichier).
//!   - `Fail2banStatusReader` : lecture seule du statut fail2ban (JSON cron).

use async_trait::async_trait;

use crate::ops::domain::entities::ip_ban::Fail2banStatus;
use crate::ops::domain::errors::DomainError;

#[async_trait]
pub trait HostBanQueue: Send + Sync {
    /// Met une IP en file de ban (appliquee au prochain tick du cron host).
    async fn queue_ban(&self, ip: &str, reason: Option<&str>) -> Result<(), DomainError>;
    /// Met une IP en file de deban.
    async fn queue_unban(&self, ip: &str, reason: Option<&str>) -> Result<(), DomainError>;
}

#[async_trait]
pub trait Fail2banStatusReader: Send + Sync {
    /// `None` si fail2ban n'est pas installe (fichier de statut absent).
    async fn read_status(&self) -> Result<Option<Fail2banStatus>, DomainError>;
}
