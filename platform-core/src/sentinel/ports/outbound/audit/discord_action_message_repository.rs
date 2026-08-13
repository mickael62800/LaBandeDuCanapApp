//! Port outbound pour le mapping `discord_action_messages` (migration 175).

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::audit::discord_action_message::DiscordActionMessage;
use crate::sentinel::domain::entities::audit::discord_action_message::NewDiscordActionMessage;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait DiscordActionMessageRepository: Send + Sync {
    /// Enregistre la correspondance (idempotent : ON CONFLICT DO NOTHING
    /// sur la cle composite `(action_id, kind)`).
    async fn register(&self, msg: NewDiscordActionMessage) -> Result<(), DomainError>;

    /// Liste tous les mappings pour une `action_id`.
    async fn list_for_action(
        &self,
        action_id: Uuid,
    ) -> Result<Vec<DiscordActionMessage>, DomainError>;
}
