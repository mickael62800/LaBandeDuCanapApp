//! Implementation du use case "bans IP". Toute la logique metier (validation
//! d'IP, refus loopback/LAN, orchestration file-shim + DB) est ici ; les
//! effets de bord passent par les ports outbound injectes.

use std::net::IpAddr;
use std::sync::Arc;

use async_trait::async_trait;

use crate::ops::domain::entities::ip_ban::{Fail2banStatus, ManualIpBan};
use crate::ops::domain::errors::DomainError;
use crate::ops::ports::inbound::manage_ip_bans::ManageIpBansUseCase;
use crate::ops::ports::outbound::host_ban_queue::{Fail2banStatusReader, HostBanQueue};
use crate::ops::ports::outbound::ip_ban_repository::IpBanRepository;

pub struct ManageIpBansService {
    repo: Arc<dyn IpBanRepository>,
    queue: Arc<dyn HostBanQueue>,
    fail2ban: Arc<dyn Fail2banStatusReader>,
}

impl ManageIpBansService {
    pub fn new(
        repo: Arc<dyn IpBanRepository>,
        queue: Arc<dyn HostBanQueue>,
        fail2ban: Arc<dyn Fail2banStatusReader>,
    ) -> Self {
        Self {
            repo,
            queue,
            fail2ban,
        }
    }
}

/// Valide une IP destinee a un ban : doit etre parsable et publique.
///
/// Bannir une adresse interne coupe l'hote de son propre reseau : le garde-fou
/// couvre donc les DEUX familles. La version initiale ne testait `is_private()`
/// que sur la branche IPv4, ce qui laissait passer une ULA (`fc00::/7`) et une
/// link-local (`fe80::/10`) — seul `::1` etait arrete, par `is_loopback`.
fn validate_bannable_ip(raw: &str) -> Result<IpAddr, DomainError> {
    let ip: IpAddr = raw
        .parse()
        .map_err(|_| DomainError::ValidationError(format!("IP invalide : {raw}")))?;
    if ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() {
        return Err(DomainError::ValidationError(
            "Refus de bannir une IP non routable (loopback, indeterminee ou multicast)".into(),
        ));
    }
    let interne = match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local() || v4.is_broadcast(),
        // `fc00::/7` (unique local) et `fe80::/10` (link-local). Les methodes
        // stables ne les exposent pas encore : on teste les prefixes.
        IpAddr::V6(v6) => {
            let o = v6.octets();
            (o[0] & 0xfe) == 0xfc || (o[0] == 0xfe && (o[1] & 0xc0) == 0x80)
        }
    };
    if interne {
        return Err(DomainError::ValidationError(
            "Refus de bannir une IP privee ou de lien local".into(),
        ));
    }
    Ok(ip)
}

#[async_trait]
impl ManageIpBansUseCase for ManageIpBansService {
    async fn ban(&self, ip: &str, reason: Option<String>, actor: &str) -> Result<(), DomainError> {
        let ip = ip.trim();
        validate_bannable_ip(ip)?;
        let reason = reason.as_deref().map(str::trim).filter(|s| !s.is_empty());

        // 1. File-shim host (applique au prochain tick du cron).
        self.queue.queue_ban(ip, reason).await?;
        // 2. Persiste/reactive le ban manuel (source de verite UI).
        self.repo.record_manual_ban(ip, actor, reason).await?;

        // Les logs de l'IP ne sont PLUS supprimes ici.
        //
        // Le ban effacait les traces qui le justifiaient : impossible ensuite
        // de reconstituer ce que l'adresse avait fait, et un aller-retour
        // ban/deban suffisait a nettoyer l'historique de n'importe quelle IP
        // sans que le journal en garde mention. Une mesure de securite ne doit
        // pas detruire ses propres preuves.
        //
        // La retention des logs est le travail de la purge programmee
        // (`/security/cleanup`), qui est explicite et auditee.
        Ok(())
    }

    async fn unban(
        &self,
        ip: &str,
        reason: Option<String>,
        actor: &str,
    ) -> Result<(), DomainError> {
        let ip = ip.trim();
        // Un deban accepte n'importe quelle IP valide (y compris privee).
        let _: IpAddr = ip
            .parse()
            .map_err(|_| DomainError::ValidationError(format!("IP invalide : {ip}")))?;
        let reason = reason.as_deref().map(str::trim).filter(|s| !s.is_empty());

        self.queue.queue_unban(ip, reason).await?;
        self.repo.mark_unbanned(ip, actor).await?;
        Ok(())
    }

    async fn list_manual_bans(&self) -> Result<Vec<ManualIpBan>, DomainError> {
        self.repo.list_active().await
    }

    async fn fail2ban_status(&self) -> Result<Option<Fail2banStatus>, DomainError> {
        self.fail2ban.read_status().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeIpBanRepo;
    #[async_trait]
    impl IpBanRepository for FakeIpBanRepo {
        async fn record_manual_ban(
            &self,
            _ip: &str,
            _actor: &str,
            _reason: Option<&str>,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn mark_unbanned(&self, _ip: &str, _actor: &str) -> Result<(), DomainError> {
            Ok(())
        }

        async fn list_active(&self) -> Result<Vec<ManualIpBan>, DomainError> {
            Ok(vec![])
        }
    }

    struct FakeHostBanQueue;
    #[async_trait]
    impl HostBanQueue for FakeHostBanQueue {
        async fn queue_ban(&self, _ip: &str, _reason: Option<&str>) -> Result<(), DomainError> {
            Ok(())
        }

        async fn queue_unban(&self, _ip: &str, _reason: Option<&str>) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct FakeFail2banReader;
    #[async_trait]
    impl Fail2banStatusReader for FakeFail2banReader {
        async fn read_status(&self) -> Result<Option<Fail2banStatus>, DomainError> {
            Ok(None)
        }
    }

    #[test]
    fn validate_public_ipv4() {
        assert!(validate_bannable_ip("8.8.8.8").is_ok());
        assert!(validate_bannable_ip("1.1.1.1").is_ok());
    }

    #[test]
    fn validate_rejects_invalid_ip() {
        assert!(validate_bannable_ip("not.an.ip").is_err());
        assert!(validate_bannable_ip("999.999.999.999").is_err());
    }

    #[test]
    fn validate_rejects_loopback() {
        assert!(validate_bannable_ip("127.0.0.1").is_err());
        assert!(validate_bannable_ip("::1").is_err());
    }

    #[test]
    fn validate_rejects_private_ipv4() {
        assert!(validate_bannable_ip("192.168.1.1").is_err());
        assert!(validate_bannable_ip("10.0.0.1").is_err());
        assert!(validate_bannable_ip("172.16.0.1").is_err());
    }

    #[test]
    fn validate_rejects_multicast() {
        assert!(validate_bannable_ip("224.0.0.1").is_err());
    }

    #[tokio::test]
    async fn ban_validates_ip_first() {
        let service = ManageIpBansService::new(
            Arc::new(FakeIpBanRepo),
            Arc::new(FakeHostBanQueue),
            Arc::new(FakeFail2banReader),
        );
        let result = service.ban("not.an.ip", None, "admin").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn unban_allows_any_valid_ip() {
        let service = ManageIpBansService::new(
            Arc::new(FakeIpBanRepo),
            Arc::new(FakeHostBanQueue),
            Arc::new(FakeFail2banReader),
        );
        let result = service.unban("192.168.1.1", None, "admin").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_manual_bans() {
        let service = ManageIpBansService::new(
            Arc::new(FakeIpBanRepo),
            Arc::new(FakeHostBanQueue),
            Arc::new(FakeFail2banReader),
        );
        let result = service.list_manual_bans().await;
        assert!(result.is_ok());
    }
}
