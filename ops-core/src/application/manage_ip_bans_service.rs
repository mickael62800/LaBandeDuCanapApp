//! Implementation du use case "bans IP". Toute la logique metier (validation
//! d'IP, refus loopback/LAN, orchestration file-shim + DB) est ici ; les
//! effets de bord passent par les ports outbound injectes.

use std::net::IpAddr;
use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::ip_ban::{Fail2banStatus, ManualIpBan};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_ip_bans::ManageIpBansUseCase;
use crate::ports::outbound::host_ban_queue::{Fail2banStatusReader, HostBanQueue};
use crate::ports::outbound::ip_ban_repository::IpBanRepository;

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
    async fn ban(
        &self,
        ip: &str,
        reason: Option<String>,
        actor: &str,
    ) -> Result<(), DomainError> {
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
