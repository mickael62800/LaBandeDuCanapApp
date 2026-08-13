//! Use case du mapping `discord_action_messages` (cf.
//! SYNC_DISCORD_WEB_DESIGN.md). Permet aux adapters inbound (HTTP, gRPC)
//! d'enregistrer / lister / supprimer un mapping action_id <-> message
//! Discord SANS appeler directement le repo outbound.

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::audit::discord_action_message::DiscordActionMessage;
use crate::sentinel::domain::entities::audit::discord_action_message::NewDiscordActionMessage;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ManageDiscordActionMessagesUseCase: Send + Sync {
    /// Enregistre un mapping (idempotent). Le `kind` doit etre non vide.
    async fn register(&self, msg: NewDiscordActionMessage) -> Result<(), DomainError>;

    /// Liste tous les mappings d'une action (toutes les representations
    /// Discord d'une meme entite metier).
    async fn list_for_action(
        &self,
        action_id: Uuid,
    ) -> Result<Vec<DiscordActionMessage>, DomainError>;
}
