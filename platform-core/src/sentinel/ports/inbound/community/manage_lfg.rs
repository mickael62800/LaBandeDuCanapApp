use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::lfg::{LfgPost, UpsertLfgCommand};
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ManageLfgUseCase: Send + Sync {
    async fn list(
        &self,
        guild_id: &str,
        live_only: bool,
        limit: i64,
    ) -> Result<Vec<LfgPost>, DomainError>;

    async fn get(&self, id: Uuid) -> Result<LfgPost, DomainError>;

    async fn create(&self, cmd: UpsertLfgCommand) -> Result<LfgPost, DomainError>;

    /// Fermeture. `actor_id` doit etre l'auteur, sauf si `is_staff` : sans ce
    /// controle, n'importe qui fermerait l'annonce d'un autre.
    async fn close(&self, id: Uuid, actor_id: &str, is_staff: bool) -> Result<(), DomainError>;

    async fn delete(&self, id: Uuid, actor_id: &str, is_staff: bool) -> Result<(), DomainError>;

    /// Se manifester. Idempotent.
    async fn join(&self, id: Uuid, user_id: &str, username: &str) -> Result<LfgPost, DomainError>;

    async fn leave(&self, id: Uuid, user_id: &str) -> Result<LfgPost, DomainError>;
}
