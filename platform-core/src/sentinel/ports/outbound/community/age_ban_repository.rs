//! Port de persistance des bans d'age (verification au reglement).

use async_trait::async_trait;

use crate::sentinel::domain::entities::community::age_ban::AgeBan;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait AgeBanRepository: Send + Sync {
    /// Enregistre un nouveau ban d'age.
    async fn create(&self, ban: &AgeBan) -> Result<(), DomainError>;

    /// Liste les bans `pending` dont la date de deban est atteinte
    /// (`unban_at <= now`), pour que le worker les leve.
    async fn list_due(&self, limit: i64) -> Result<Vec<AgeBan>, DomainError>;

    /// Marque un ban comme `lifted` (deban effectue).
    async fn mark_lifted(&self, id: uuid::Uuid) -> Result<(), DomainError>;
}
