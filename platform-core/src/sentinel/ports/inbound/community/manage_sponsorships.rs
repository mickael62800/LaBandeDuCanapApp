use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::outbound::community::sponsorship_repository::Sponsorship;
use crate::sentinel::ports::outbound::community::temp_role_repository::TempRole;

/// Use case du domaine Community pour les parrainages (sponsorships) et les
/// roles temporaires (temp-roles). Regroupe la persistance derriere les ports
/// sortants ; la validation de format (ex: `expires_at` RFC3339) vit ici.
#[async_trait]
pub trait ManageSponsorshipsUseCase: Send + Sync {
    /// Enregistre un parrainage (idempotent sur `(guild_id, sponsored_id)`).
    async fn create_sponsorship(
        &self,
        guild_id: &str,
        sponsor_id: &str,
        sponsored_id: &str,
    ) -> Result<(), DomainError>;

    /// Liste les parrainages d'une guilde (plus recents d'abord).
    async fn list_sponsorships(&self, guild_id: &str) -> Result<Vec<Sponsorship>, DomainError>;

    /// Cree/prolonge un role temporaire. `expires_at` doit etre au format
    /// RFC3339 (valide par le use case).
    async fn create_temp_role(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
        expires_at: &str,
    ) -> Result<(), DomainError>;

    /// Liste les roles temporaires encore actifs (expiration future).
    async fn list_temp_roles(&self, guild_id: &str) -> Result<Vec<TempRole>, DomainError>;

    /// Supprime un role temporaire.
    async fn delete_temp_role(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
    ) -> Result<(), DomainError>;
}
