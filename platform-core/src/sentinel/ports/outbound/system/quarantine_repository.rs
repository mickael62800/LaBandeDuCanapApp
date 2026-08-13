//! Port outbound : persistance des quarantaines de securite
//! (`security_quarantine_pending`). Tout le SQL vit dans l'adapter Postgres.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::sentinel::domain::entities::system::quarantine::ActiveQuarantine;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait QuarantineRepository: Send + Sync {
    /// UPSERT idempotent : (re)pose la date d'expiration d'un membre en
    /// quarantaine (re-quarantaine reset le timer).
    async fn upsert(
        &self,
        guild_id: &str,
        user_id: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), DomainError>;

    /// Liste les quarantaines encore actives (expires_at > now).
    async fn list_active(&self) -> Result<Vec<ActiveQuarantine>, DomainError>;

    /// Supprime la quarantaine d'un membre (idempotent).
    async fn delete(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
}
