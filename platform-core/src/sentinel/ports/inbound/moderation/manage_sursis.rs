//! Use case : gestion des « bans en sursis ».

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::moderation::sursis::{Sursis, SursisStatus};
use crate::sentinel::domain::errors::DomainError;

/// Donnees de mise en sursis (le bot fournit les roles sauvegardes et le salon).
pub struct CreateSursisCommand {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub reason: String,
    pub saved_roles: Vec<String>,
    pub channel_id: Option<String>,
    /// Delai avant ban definitif (jours).
    pub days: i64,
}

#[async_trait]
pub trait ManageSursisUseCase: Send + Sync {
    async fn create(&self, cmd: CreateSursisCommand) -> Result<Sursis, DomainError>;
    async fn get(&self, id: Uuid) -> Result<Option<Sursis>, DomainError>;
    async fn resolve(&self, id: Uuid, status: SursisStatus) -> Result<bool, DomainError>;
    /// Sursis echus (worker).
    async fn list_due(&self) -> Result<Vec<Sursis>, DomainError>;
}
