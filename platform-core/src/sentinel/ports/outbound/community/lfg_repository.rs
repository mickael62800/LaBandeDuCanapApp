use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::lfg::{LfgPost, UpsertLfgCommand};
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait LfgRepository: Send + Sync {
    /// Annonces d'une guilde, les plus recentes d'abord, avec leurs
    /// interesses deja charges.
    ///
    /// `live_only` sert la page publique : elle n'affiche ni les annonces
    /// fermees, ni les expirees. Le back-office, lui, veut tout voir pour
    /// pouvoir moderer une annonce close.
    async fn list(
        &self,
        guild_id: &str,
        live_only: bool,
        limit: i64,
    ) -> Result<Vec<LfgPost>, DomainError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<LfgPost>, DomainError>;

    async fn create(&self, cmd: &UpsertLfgCommand) -> Result<LfgPost, DomainError>;

    /// Fermeture manuelle par l'auteur ou le staff. Ne supprime pas :
    /// l'annonce reste consultable un temps.
    async fn set_open(&self, id: Uuid, open: bool) -> Result<bool, DomainError>;

    async fn delete(&self, id: Uuid) -> Result<bool, DomainError>;

    /// Idempotent : se manifester deux fois ne cree pas deux entrees.
    async fn add_interest(
        &self,
        id: Uuid,
        user_id: &str,
        username: &str,
    ) -> Result<(), DomainError>;

    async fn remove_interest(&self, id: Uuid, user_id: &str) -> Result<bool, DomainError>;

    /// Purge des annonces expirees depuis un moment. Appelee par le worker :
    /// sans elle la table grossit indefiniment, meme si l'affichage les
    /// filtre deja.
    async fn purge_expired(&self, older_than_hours: i64) -> Result<u64, DomainError>;
}
