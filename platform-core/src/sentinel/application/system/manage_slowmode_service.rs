//! Use case Slowmode : calcule la date d'expiration et delegue la persistance au
//! repo. Toute la regle metier vit ici ; le SQL dans `SlowmodeRepository`, le
//! handler HTTP ne fait que parser/RBAC/mapper.

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::system::manage_slowmode::ManageSlowmodeUseCase;
use crate::sentinel::ports::outbound::system::slowmode_repository::SlowmodeRepository;

pub struct ManageSlowmodeService {
    repo: Arc<dyn SlowmodeRepository>,
}

impl ManageSlowmodeService {
    pub fn new(repo: Arc<dyn SlowmodeRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageSlowmodeUseCase for ManageSlowmodeService {
    async fn activate(
        &self,
        guild_id: &str,
        previous_states: serde_json::Value,
        duration_secs: i64,
        imposed_rate: i32,
    ) -> Result<(), DomainError> {
        // Au moins 1s de duree (anti-config abusive / valeur nulle).
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(duration_secs.max(1));
        self.repo
            .upsert(guild_id, &previous_states, expires_at, imposed_rate)
            .await
    }

    async fn deactivate(&self, guild_id: &str) -> Result<(), DomainError> {
        self.repo.delete(guild_id).await
    }
}

#[cfg(test)]
#[path = "tests/manage_slowmode.rs"]
mod tests;
