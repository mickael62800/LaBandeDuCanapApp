//! Use case Quarantaine : calcule la date d'expiration (delai avant kick) et
//! delegue la persistance au repo. Toute la regle metier vit ici ; le SQL dans
//! `QuarantineRepository`, le handler HTTP ne fait que parser/RBAC/mapper.

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::entities::system::quarantine::ActiveQuarantine;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::system::manage_quarantine::ManageQuarantineUseCase;
use crate::sentinel::ports::outbound::system::quarantine_repository::QuarantineRepository;

pub struct ManageQuarantineService {
    repo: Arc<dyn QuarantineRepository>,
}

impl ManageQuarantineService {
    pub fn new(repo: Arc<dyn QuarantineRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageQuarantineUseCase for ManageQuarantineService {
    async fn quarantine_user(
        &self,
        guild_id: &str,
        user_id: &str,
        timeout_secs: i64,
    ) -> Result<(), DomainError> {
        // Au moins 1s de delai avant kick (anti-config abusive / valeur nulle).
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(timeout_secs.max(1));
        self.repo.upsert(guild_id, user_id, expires_at).await
    }

    async fn list_active(&self) -> Result<Vec<ActiveQuarantine>, DomainError> {
        self.repo.list_active().await
    }

    async fn lift(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.repo.delete(guild_id, user_id).await
    }
}
