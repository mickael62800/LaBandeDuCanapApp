//! Port outbound : persistance des salons en slowmode adaptatif actif (BUG3).

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait AdaptiveSlowmodeRepository: Send + Sync {
    /// Marque un salon comme ayant un slowmode adaptatif actif (upsert).
    async fn mark(&self, guild_id: &str, channel_id: &str) -> Result<(), DomainError>;

    /// Retire un salon (slowmode desactive). Cle par channel_id (unique).
    async fn unmark(&self, channel_id: &str) -> Result<(), DomainError>;

    /// Tous les salons actifs, pour rechargement au demarrage du bot.
    async fn list_all(&self) -> Result<Vec<(String, String)>, DomainError>;
}
