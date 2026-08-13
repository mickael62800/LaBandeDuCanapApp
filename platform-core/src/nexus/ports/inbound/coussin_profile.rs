use crate::nexus::{
    domain::errors::DomainError,
    ports::outbound::coussin_repository::{CoussinCombat, CoussinProfile},
};
use async_trait::async_trait;
#[async_trait]
pub trait CoussinProfileUseCase: Send + Sync {
    async fn profile(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<CoussinProfile, DomainError>;
    /// Derniers combats resolus du joueur. Lecture seule.
    async fn combat_history(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<
        Vec<crate::nexus::ports::outbound::coussin_repository::CoussinCombatResult>,
        DomainError,
    >;

    async fn choose_class(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        class: &str,
    ) -> Result<CoussinProfile, DomainError>;
    async fn train(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        stat: &str,
    ) -> Result<CoussinProfile, DomainError>;
    /// Classement des joueurs de la guild (supervision). Lecture seule :
    /// contrairement a `profile`, ne cree aucun profil manquant.
    async fn ranking(&self, guild_id: &str, limit: i64)
        -> Result<Vec<CoussinProfile>, DomainError>;
}

#[async_trait]
pub trait CoussinCombatUseCase: Send + Sync {
    async fn challenge(
        &self,
        guild_id: &str,
        channel_id: &str,
        attacker_id: &str,
        attacker_name: &str,
        defender_id: &str,
        defender_name: &str,
        mise: i64,
    ) -> Result<CoussinCombat, DomainError>;
    async fn accept(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError>;
    async fn refuse(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError>;
    async fn resolve(&self, id: uuid::Uuid) -> Result<bool, DomainError>;
}
