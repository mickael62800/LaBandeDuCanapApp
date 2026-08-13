//! Use case "bans IP" du panel securite. Valide les IPs, pousse l'ordre vers
//! le file-shim host, persiste l'historique en DB et lit le statut fail2ban.
//! Aucune logique infra ici : tout passe par les ports outbound.

use async_trait::async_trait;

use crate::ops::domain::entities::ip_ban::{Fail2banStatus, ManualIpBan};
use crate::ops::domain::errors::DomainError;

#[async_trait]
pub trait ManageIpBansUseCase: Send + Sync {
    /// Bannit une IP : valide, met en file host, persiste. Ne purge PAS les
    /// logs de l'IP — cf. le commentaire de `ban` dans le service.
    async fn ban(&self, ip: &str, reason: Option<String>, actor: &str) -> Result<(), DomainError>;

    /// Leve un ban : valide, met en file host, marque la ligne DB comme levee.
    async fn unban(&self, ip: &str, reason: Option<String>, actor: &str)
        -> Result<(), DomainError>;

    /// Liste des bans manuels actifs (non leves).
    async fn list_manual_bans(&self) -> Result<Vec<ManualIpBan>, DomainError>;

    /// Statut fail2ban (None si non installe / fichier absent).
    async fn fail2ban_status(&self) -> Result<Option<Fail2banStatus>, DomainError>;
}
