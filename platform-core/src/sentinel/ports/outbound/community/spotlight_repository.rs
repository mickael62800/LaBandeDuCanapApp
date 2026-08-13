use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::spotlight::{Spotlight, UpsertSpotlightCommand};
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait SpotlightRepository: Send + Sync {
    /// Membre du mois d'une periode donnee (`YYYY-MM`).
    async fn find_by_period(
        &self,
        guild_id: &str,
        period: &str,
    ) -> Result<Option<Spotlight>, DomainError>;

    /// Le plus recent, quelle que soit sa periode.
    ///
    /// La page ne peut pas se contenter du mois courant : si le staff n'a pas
    /// encore designe personne en debut de mois, la section disparaitrait au
    /// lieu de continuer a mettre en avant celui du mois precedent.
    async fn find_latest(&self, guild_id: &str) -> Result<Option<Spotlight>, DomainError>;

    async fn list(&self, guild_id: &str, limit: i64) -> Result<Vec<Spotlight>, DomainError>;

    /// Un seul membre par periode : redesigner remplace.
    async fn upsert(&self, cmd: &UpsertSpotlightCommand) -> Result<Spotlight, DomainError>;

    async fn delete(&self, id: Uuid) -> Result<bool, DomainError>;
}
