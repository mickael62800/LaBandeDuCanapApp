use async_trait::async_trait;

use crate::sentinel::domain::entities::community::level::UserLevel;
use crate::sentinel::domain::entities::community::level::XpSource;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use crate::sentinel::domain::errors::DomainError;

pub struct AddXpCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub amount: i64,
    pub source: XpSource,
}

pub struct AddXpResult {
    pub user_level: UserLevel,
    /// `true` si le niveau de la source (texte ou vocal) a augmente.
    pub leveled_up: bool,
    /// Ancien niveau de la source declenchante (texte ou vocal).
    pub old_level: i32,
    /// Ancien niveau global (= level_from_xp(xp_text + xp_voice) avant l'ajout).
    /// Sert au bot pour declencher le renommage `[NN]Pseudo` uniquement
    /// quand le niveau total change reellement.
    pub old_level_global: i32,
    pub source: XpSource,
}

/// Fait brut : un message qualifiant a eu lieu. L'API calcule le montant
/// d'XP (base x multiplicateurs channel/role x streak, clampe), applique le
/// cooldown anti-farm et met a jour le streak — le bot n'envoie que le fait.
pub struct RecordTextActivityCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub channel_id: u64,
    pub role_ids: Vec<u64>,
}

/// Fait brut : `seconds` secondes vocales creditables dans `channel_id`.
/// L'API calcule le montant d'XP (base x multiplicateurs, clampe).
pub struct RecordVoiceActivityCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub channel_id: u64,
    pub role_ids: Vec<u64>,
    pub seconds: u64,
}

/// Resultat d'un fait d'activite (texte ou vocal).
pub struct RecordActivityResult {
    /// `true` si aucun XP n'a ete credite (cooldown, module desactive,
    /// montant nul) : le bot ne doit rien afficher.
    pub skipped: bool,
    /// XP effectivement credite (0 si `skipped`).
    pub xp_gained: i64,
    pub user_level: UserLevel,
    /// `true` si le niveau de la source (texte ou vocal) a augmente.
    pub leveled_up: bool,
    /// Ancien niveau de la source declenchante.
    pub old_level: i32,
    /// Ancien niveau global (avant l'ajout) — pour le renommage `[NN]Pseudo`.
    pub old_level_global: i32,
    pub source: XpSource,
    /// Streak courant (jours consecutifs) apres traitement.
    pub streak_current: u32,
}

/// Set la valeur exacte de l'XP texte et/ou voix d'un utilisateur.
/// `None` = ne pas modifier ce champ. Les niveaux sont recalcules
/// automatiquement depuis les nouvelles valeurs d'XP.
pub struct SetUserXpCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub xp_text: Option<i64>,
    pub xp_voice: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetTarget {
    All,
    Text,
    Voice,
}

#[async_trait]
pub trait ManageLevelsUseCase: Send + Sync {
    async fn add_xp(&self, cmd: AddXpCommand) -> Result<AddXpResult, DomainError>;
    /// Enregistre un fait d'activite TEXTE et calcule tout l'XP server-side
    /// (config serveur + cooldown + streak + multiplicateurs).
    async fn record_text_activity(
        &self,
        cmd: RecordTextActivityCommand,
    ) -> Result<RecordActivityResult, DomainError>;
    /// Enregistre un fait d'activite VOCALE (secondes brutes) et calcule
    /// l'XP server-side (config serveur + multiplicateurs).
    async fn record_voice_activity(
        &self,
        cmd: RecordVoiceActivityCommand,
    ) -> Result<RecordActivityResult, DomainError>;
    async fn get_user_level(&self, guild_id: &str, user_id: &str)
        -> Result<UserLevel, DomainError>;
    async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError>;
    async fn get_leaderboard_by_source(
        &self,
        guild_id: &str,
        source: XpSource,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError>;
    /// Set valeur exacte XP texte/voix (admin override). Recalcule les niveaux.
    async fn set_user_xp(&self, cmd: SetUserXpCommand) -> Result<UserLevel, DomainError>;
    /// Reset XP a 0 sur la cible (text / voice / all). Recalcule les niveaux.
    async fn reset_user_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        target: ResetTarget,
    ) -> Result<UserLevel, DomainError>;
}
