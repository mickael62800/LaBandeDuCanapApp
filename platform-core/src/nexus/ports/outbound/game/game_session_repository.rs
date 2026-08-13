//! Ports de persistance des "evenements de serveur" : reglages par template
//! (role a pinguer) et inscriptions des joueurs a une session.

use async_trait::async_trait;
use uuid::Uuid;

use crate::nexus::domain::entities::game::session::{
    GameSessionRegistration, GameTemplateSettings,
};
use crate::nexus::domain::errors::DomainError;

#[async_trait]
pub trait GameTemplateSettingsRepository: Send + Sync {
    /// Reglages (guild, template) — None si jamais configure.
    async fn get(
        &self,
        guild_id: &str,
        template_slug: &str,
    ) -> Result<Option<GameTemplateSettings>, DomainError>;

    /// Tous les reglages de templates d'une guild.
    async fn list(&self, guild_id: &str) -> Result<Vec<GameTemplateSettings>, DomainError>;

    /// Upsert du role a pinguer pour un template sur une guild.
    async fn set_role(
        &self,
        guild_id: &str,
        template_slug: &str,
        discord_role_id: Option<&str>,
    ) -> Result<(), DomainError>;
}

#[async_trait]
pub trait GameSessionRegistrationRepository: Send + Sync {
    /// Inscrit un joueur (idempotent : re-inscription = no-op).
    async fn register(&self, server_id: Uuid, user_id: &str) -> Result<(), DomainError>;

    /// Desinscrit un joueur.
    async fn unregister(&self, server_id: Uuid, user_id: &str) -> Result<(), DomainError>;

    /// Liste les inscrits d'une session.
    async fn list(&self, server_id: Uuid) -> Result<Vec<GameSessionRegistration>, DomainError>;
}
