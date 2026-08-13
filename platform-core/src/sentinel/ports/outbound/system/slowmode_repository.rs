//! Port outbound : persistance du slowmode de securite manuel
//! (`security_slowmode_active`). Tout le SQL vit dans l'adapter Postgres.
//! Distinct de l'automod adaptatif (`AdaptiveSlowmodeRepository`, moderation).

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait SlowmodeRepository: Send + Sync {
    /// UPSERT idempotent : (re)pose les rates d'origine par salon
    /// (`previous_states`), la date d'expiration et le rate impose par le raid
    /// (re-activation reset le timer).
    async fn upsert(
        &self,
        guild_id: &str,
        previous_states: &serde_json::Value,
        expires_at: DateTime<Utc>,
        imposed_rate: i32,
    ) -> Result<(), DomainError>;

    /// Supprime le slowmode d'une guild (idempotent).
    async fn delete(&self, guild_id: &str) -> Result<(), DomainError>;
}
