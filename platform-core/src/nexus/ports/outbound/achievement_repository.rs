//! Persistance des hauts faits : catalogue, liaisons et attributions.

use async_trait::async_trait;
use uuid::Uuid;

use crate::nexus::domain::entities::achievement::{
    Achievement, GameIdentity, GamePlayerLink, UserAchievement,
};
use crate::nexus::domain::errors::DomainError;

/// Champs modifiables d'une definition depuis le dashboard. `None` = ne pas
/// toucher, ce qui permet de ne changer que l'image sans reecrire le reste.
#[derive(Debug, Clone, Default)]
pub struct AchievementUpdate {
    pub icon_url: Option<Option<String>>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub hidden: Option<bool>,
    pub criteria: Option<serde_json::Value>,
}

impl AchievementUpdate {
    pub fn is_empty(&self) -> bool {
        self.icon_url.is_none()
            && self.name.is_none()
            && self.description.is_none()
            && self.enabled.is_none()
            && self.hidden.is_none()
            && self.criteria.is_none()
    }
}

#[async_trait]
pub trait AchievementRepository: Send + Sync {
    // ── Catalogue ──────────────────────────────────────────────────────
    /// Definitions, filtrees par jeu quand `game` est fourni.
    async fn list_definitions(&self, game: Option<&str>) -> Result<Vec<Achievement>, DomainError>;
    async fn find_definition(&self, id: Uuid) -> Result<Option<Achievement>, DomainError>;
    async fn find_definition_by_code(
        &self,
        game: Option<&str>,
        code: &str,
    ) -> Result<Option<Achievement>, DomainError>;
    async fn update_definition(
        &self,
        id: Uuid,
        update: AchievementUpdate,
    ) -> Result<Achievement, DomainError>;

    // ── Liaisons d'identite de jeu ─────────────────────────────────────
    async fn find_link(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        game: &str,
    ) -> Result<Option<GamePlayerLink>, DomainError>;
    /// Resolution inverse : quel membre porte cette identite de jeu ?
    async fn find_link_by_player(
        &self,
        guild_id: &str,
        identity: &GameIdentity,
    ) -> Result<Option<GamePlayerLink>, DomainError>;
    /// Enregistre (ou remplace) la liaison d'un membre pour un jeu.
    ///
    /// Renvoie `Conflict` si l'identite est deja revendiquee par un autre
    /// membre de la guilde : c'est la protection contre l'usurpation.
    async fn upsert_link(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        identity: &GameIdentity,
        verified: bool,
    ) -> Result<GamePlayerLink, DomainError>;
    async fn delete_link(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        game: &str,
    ) -> Result<bool, DomainError>;

    // ── Attributions ───────────────────────────────────────────────────
    /// Attribue le haut fait. Renvoie `None` si le membre le possedait deja
    /// ou si `source_event_id` a deja ete consomme : l'appelant sait alors
    /// qu'il ne doit RIEN publier (idempotence).
    async fn insert_unlock(
        &self,
        unlock: &UserAchievement,
    ) -> Result<Option<UserAchievement>, DomainError>;
    async fn list_for_member(
        &self,
        guild_id: &str,
        discord_user_id: &str,
    ) -> Result<Vec<UserAchievement>, DomainError>;
    async fn count_for_member(
        &self,
        guild_id: &str,
        discord_user_id: &str,
    ) -> Result<i64, DomainError>;
}
