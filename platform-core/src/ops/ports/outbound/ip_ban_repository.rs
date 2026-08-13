//! Port outbound : persistance des bans IP manuels (table `manual_ip_bans`).
//!
//! `delete_api_logs_for_ip` a ete retire : bannir une IP ne purge plus ses
//! logs, et le port n'avait donc plus d'appelant. Le laisser en place rendait
//! la capacite triviale a rebrancher par inadvertance.

use async_trait::async_trait;

use crate::ops::domain::entities::ip_ban::ManualIpBan;
use crate::ops::domain::errors::DomainError;

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
}
