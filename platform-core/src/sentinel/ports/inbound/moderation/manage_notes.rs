use async_trait::async_trait;

use crate::sentinel::domain::entities::moderation::user_note::UserNote;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use crate::sentinel::domain::errors::DomainError;

pub struct AddNoteCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub category: String,
}

#[async_trait]
pub trait ManageNotesUseCase: Send + Sync {
    async fn add_note(&self, cmd: AddNoteCommand) -> Result<UserNote, DomainError>;
    async fn get_notes(&self, guild_id: &str, user_id: &str) -> Result<Vec<UserNote>, DomainError>;
    async fn delete_note(&self, note_id: &str) -> Result<(), DomainError>;
    /// Guilde d'une note par son id (pour la garde RBAC cote handler). `None` si
    /// la note n'existe pas.
    async fn note_guild_id(&self, note_id: &str) -> Result<Option<String>, DomainError>;
}
