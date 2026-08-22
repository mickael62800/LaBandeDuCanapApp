//! Use case Lockdown : calcule la date d'expiration et delegue la persistance au
//! repo. Toute la regle metier vit ici ; le SQL dans `LockdownRepository`, le
//! handler HTTP ne fait que parser/RBAC/mapper.

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::system::manage_lockdown::ManageLockdownUseCase;
use crate::sentinel::ports::outbound::system::lockdown_repository::LockdownRepository;

pub struct ManageLockdownService {
    repo: Arc<dyn LockdownRepository>,
}

impl ManageLockdownService {
    pub fn new(repo: Arc<dyn LockdownRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageLockdownUseCase for ManageLockdownService {
    async fn activate(
        &self,
        guild_id: &str,
        saved_states: serde_json::Value,
        duration_secs: i64,
    ) -> Result<(), DomainError> {
        // Au moins 1s de duree (anti-config abusive / valeur nulle).
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(duration_secs.max(1));
        self.repo.upsert(guild_id, &saved_states, expires_at).await
    }

    async fn deactivate(&self, guild_id: &str) -> Result<(), DomainError> {
        self.repo.delete(guild_id).await
    }
}


