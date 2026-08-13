use async_trait::async_trait;

use crate::sentinel::domain::entities::system::guild::Guild;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait GuildRepository: Send + Sync {
    async fn upsert(&self, guild: &Guild) -> Result<(), DomainError>;
    async fn find_all(&self) -> Result<Vec<Guild>, DomainError>;
    async fn find_by_id(&self, guild_id: &str) -> Result<Option<Guild>, DomainError>;
    /// Supprime un serveur (le bot a ete retire de la guild).
    async fn delete(&self, guild_id: &str) -> Result<(), DomainError>;
    /// Reconciliation : supprime toutes les guilds absentes de `keep_ids`
    /// (le bot n'en fait plus partie). Retourne le nombre de lignes supprimees.
    /// Garde de securite : si `keep_ids` est vide, ne supprime rien.
    async fn delete_absent(&self, keep_ids: &[String]) -> Result<u64, DomainError>;
}
