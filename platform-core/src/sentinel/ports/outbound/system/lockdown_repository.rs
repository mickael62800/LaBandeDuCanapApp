//! Port outbound : persistance du lockdown de securite
//! (`security_lockdown_active`). Tout le SQL vit dans l'adapter Postgres.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait LockdownRepository: Send + Sync {
    /// UPSERT idempotent : (re)pose les states sauvegardes + la date
    /// d'expiration d'un lockdown (re-activation reset le timer).
    async fn upsert(
        &self,
        guild_id: &str,
        saved_states: &serde_json::Value,
        expires_at: DateTime<Utc>,
    ) -> Result<(), DomainError>;

    /// Supprime le lockdown d'une guild (idempotent).
    async fn delete(&self, guild_id: &str) -> Result<(), DomainError>;
}
