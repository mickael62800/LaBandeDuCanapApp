//! Port outbound : persistance des « bans en sursis ».

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::sentinel::domain::entities::moderation::sursis::{Sursis, SursisStatus};
use crate::sentinel::domain::errors::DomainError;

/// Parametres de creation d'un sursis.
pub struct NewSursis<'a> {
    pub guild_id: &'a str,
    pub user_id: &'a str,
    pub username: &'a str,
    pub moderator_id: &'a str,
    pub moderator_name: &'a str,
    pub reason: &'a str,
    pub saved_roles: Vec<String>,
    pub channel_id: Option<&'a str>,
    pub expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait SursisRepository: Send + Sync {
    /// Cree un sursis. Renvoie `Conflict` si le membre est deja en sursis.
    async fn create(&self, new: NewSursis<'_>) -> Result<Sursis, DomainError>;

    async fn get(&self, id: Uuid) -> Result<Option<Sursis>, DomainError>;

    /// Fige le statut d'un sursis (gracie / banni).
    async fn set_status(&self, id: Uuid, status: SursisStatus) -> Result<bool, DomainError>;

    /// Sursis arrives a echeance (scan worker).
    async fn list_due(&self, now: DateTime<Utc>) -> Result<Vec<Sursis>, DomainError>;
}
