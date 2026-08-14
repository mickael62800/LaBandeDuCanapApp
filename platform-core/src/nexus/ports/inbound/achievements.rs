//! Cas d'usage des hauts faits, utilises par les handlers HTTP et le bot.

use async_trait::async_trait;
use uuid::Uuid;

use crate::nexus::domain::entities::achievement::{
    Achievement, AchievementProgress, GamePlayerLink,
};
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::outbound::achievement_repository::AchievementUpdate;

/// Resultat d'une demande d'attribution.
#[derive(Debug, Clone)]
pub enum UnlockOutcome {
    /// Haut fait attribue a l'instant : l'annonce doit etre publiee.
    Unlocked(Box<UnlockedAchievement>),
    /// Deja possede, ou evenement deja consomme. Rien a publier.
    AlreadyOwned,
}

#[derive(Debug, Clone)]
pub struct UnlockedAchievement {
    pub achievement: Achievement,
    pub guild_id: String,
    pub discord_user_id: String,
    pub game_player_id: Option<String>,
    pub source_event_id: Option<String>,
}

/// Demande d'attribution issue d'un adaptateur de jeu.
#[derive(Debug, Clone)]
pub struct GameUnlockCommand {
    pub guild_id: String,
    pub game: String,
    /// Identite DANS LE JEU. Le membre Discord est resolu par la liaison
    /// verifiee ; il n'est jamais fourni par l'adaptateur.
    pub game_player_id: String,
    pub achievement_code: String,
    /// Identifiant d'evenement source, garant de l'idempotence.
    pub source_event_id: String,
}

#[async_trait]
pub trait ManageAchievementsUseCase: Send + Sync {
    // ── Catalogue ──────────────────────────────────────────────────────
    async fn list_definitions(&self, game: Option<&str>) -> Result<Vec<Achievement>, DomainError>;

    /// Mise a jour d'une definition depuis le dashboard (image, libelles,
    /// activation, seuils).
    async fn update_definition(
        &self,
        id: Uuid,
        update: AchievementUpdate,
    ) -> Result<Achievement, DomainError>;

    // ── Consultation ───────────────────────────────────────────────────
    /// Hauts faits d'un membre : catalogue + date de deblocage. Les hauts
    /// faits `hidden` non debloques sont retires de la liste.
    async fn member_progress(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        game: Option<&str>,
    ) -> Result<Vec<AchievementProgress>, DomainError>;

    // ── Liaison d'identite de jeu ──────────────────────────────────────
    /// Enregistre l'identite de jeu d'un membre (ex. SteamID64 pour Palworld).
    async fn link_identity(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        game: &str,
        game_player_id: &str,
    ) -> Result<GamePlayerLink, DomainError>;
    async fn find_link(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        game: &str,
    ) -> Result<Option<GamePlayerLink>, DomainError>;
    async fn unlink_identity(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        game: &str,
    ) -> Result<bool, DomainError>;

    // ── Attribution ────────────────────────────────────────────────────
    /// Attribution depuis un evenement de jeu. Exige une liaison VERIFIEE :
    /// sans elle, rien n'est attribue.
    async fn unlock_from_game_event(
        &self,
        cmd: GameUnlockCommand,
    ) -> Result<UnlockOutcome, DomainError>;

    /// Attribution manuelle par un administrateur (tracee par `granted_by`).
    /// Seule voie pour les hauts faits `manual`.
    async fn grant_manually(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        achievement_id: Uuid,
        granted_by: &str,
    ) -> Result<UnlockOutcome, DomainError>;
}
