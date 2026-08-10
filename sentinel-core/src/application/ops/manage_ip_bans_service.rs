//! Implementation du use case "bans IP". Toute la logique metier (validation
//! d'IP, refus loopback/LAN, orchestration file-shim + DB) est ici ; les
//! effets de bord passent par les ports outbound injectes.

use std::net::IpAddr;
use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::ops::ip_ban::{BanIpOutcome, Fail2banStatus, ManualIpBan};
use crate::domain::errors::DomainError;
use crate::ports::inbound::ops::manage_ip_bans::ManageIpBansUseCase;
use crate::ports::outbound::ops::host_ban_queue::{Fail2banStatusReader, HostBanQueue};
use crate::ports::outbound::ops::ip_ban_repository::IpBanRepository;

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
fn validate_bannable_ip(raw: &str) -> Result<IpAddr, DomainError> {
    let ip: IpAddr = raw
        .parse()
        .map_err(|_| DomainError::ValidationError(format!("IP invalide : {raw}")))?;
    if ip.is_loopback() {
        return Err(DomainError::ValidationError(
            "Refus de bannir une IP loopback".into(),
        ));
    }
    if let IpAddr::V4(v4) = ip {
        if v4.is_private() {
            return Err(DomainError::ValidationError(
                "Refus de bannir une IP privee LAN".into(),
            ));
        }
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
    ) -> Result<BanIpOutcome, DomainError> {
        let ip = ip.trim();
        validate_bannable_ip(ip)?;
        let reason = reason.as_deref().map(str::trim).filter(|s| !s.is_empty());

        // 1. File-shim host (applique au prochain tick du cron).
        self.queue.queue_ban(ip, reason).await?;
        // 2. Persiste/reactive le ban manuel (source de verite UI).
        self.repo.record_manual_ban(ip, actor, reason).await?;
        // 3. Purge des logs API de cette IP. Best-effort : on n'echoue pas.
        let deleted_logs = self.repo.delete_api_logs_for_ip(ip).await.unwrap_or(0);

        Ok(BanIpOutcome { deleted_logs })
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
