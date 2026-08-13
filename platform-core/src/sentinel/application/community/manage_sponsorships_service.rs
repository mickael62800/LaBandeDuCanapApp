//! Use case Community : parrainages + roles temporaires. Toute la persistance
//! passe par les ports sortants ; la seule regle metier locale est la
//! validation du format RFC3339 de `expires_at`.

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_sponsorships::ManageSponsorshipsUseCase;
use crate::sentinel::ports::outbound::community::sponsorship_repository::{
    Sponsorship, SponsorshipRepository,
};
use crate::sentinel::ports::outbound::community::temp_role_repository::{
    TempRole, TempRoleRepository,
};

pub struct ManageSponsorshipsService {
    sponsorships: Arc<dyn SponsorshipRepository>,
    temp_roles: Arc<dyn TempRoleRepository>,
}

impl ManageSponsorshipsService {
    pub fn new(
        sponsorships: Arc<dyn SponsorshipRepository>,
        temp_roles: Arc<dyn TempRoleRepository>,
    ) -> Self {
        Self {
            sponsorships,
            temp_roles,
        }
    }
}

#[async_trait]
impl ManageSponsorshipsUseCase for ManageSponsorshipsService {
    async fn create_sponsorship(
        &self,
        guild_id: &str,
        sponsor_id: &str,
        sponsored_id: &str,
    ) -> Result<(), DomainError> {
        self.sponsorships
            .create(guild_id, sponsor_id, sponsored_id)
            .await
    }

    async fn list_sponsorships(&self, guild_id: &str) -> Result<Vec<Sponsorship>, DomainError> {
        self.sponsorships.list(guild_id).await
    }

    async fn create_temp_role(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
        expires_at: &str,
    ) -> Result<(), DomainError> {
        // Valider le format RFC3339 avant de toucher a Postgres.
        chrono::DateTime::parse_from_rfc3339(expires_at).map_err(|_| {
            DomainError::ValidationError("expires_at doit etre au format RFC3339".into())
        })?;
        self.temp_roles
            .create(guild_id, user_id, role_id, expires_at)
            .await
    }

    async fn list_temp_roles(&self, guild_id: &str) -> Result<Vec<TempRole>, DomainError> {
        self.temp_roles.list_active(guild_id).await
    }

    async fn delete_temp_role(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
    ) -> Result<(), DomainError> {
        self.temp_roles.delete(guild_id, user_id, role_id).await
    }
}
