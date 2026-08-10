//! Port outbound : persistance des bans IP manuels (table `manual_ip_bans`)
//! et purge des logs API associes.

use async_trait::async_trait;

use crate::domain::entities::ops::ip_ban::ManualIpBan;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait IpBanRepository: Send + Sync {
    /// Insere/reactive un ban manuel (upsert : remet `unbanned_at` a NULL).
    async fn record_manual_ban(
        &self,
        ip: &str,
        banned_by: &str,
        reason: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Marque un ban comme leve (best-effort : l'IP peut venir de fail2ban
    /// uniquement, auquel cas aucune ligne n'est mise a jour).
    async fn mark_unbanned(&self, ip: &str, unbanned_by: &str) -> Result<(), DomainError>;

    /// Bans manuels actifs (non leves), du plus recent au plus ancien.
    async fn list_active(&self) -> Result<Vec<ManualIpBan>, DomainError>;

    /// Purge les logs API lies a une IP bannie. Retourne le nb de lignes.
    async fn delete_api_logs_for_ip(&self, ip: &str) -> Result<u64, DomainError>;
}
