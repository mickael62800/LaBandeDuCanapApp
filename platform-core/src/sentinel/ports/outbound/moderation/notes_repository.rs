use async_trait::async_trait;

use crate::sentinel::domain::entities::moderation::user_note::UserNote;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait NotesRepository: Send + Sync {
    async fn save(&self, note: &UserNote) -> Result<(), DomainError>;
    async fn find_by_user(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<UserNote>, DomainError>;
    async fn delete(&self, note_id: &str) -> Result<(), DomainError>;
    /// Guilde proprietaire d'une note (pour scoper la garde RBAC quand seul l'id
    /// de la note est connu). `None` si la note n'existe pas.
    async fn find_guild_id(&self, note_id: &str) -> Result<Option<String>, DomainError>;
}
