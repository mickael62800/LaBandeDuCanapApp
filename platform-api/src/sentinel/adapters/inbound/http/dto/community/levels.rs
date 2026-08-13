use platform_core::sentinel::domain::entities::community::level::xp_progress;
use platform_core::sentinel::domain::entities::community::level::UserLevel;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;
use platform_core::sentinel::ports::inbound::community::manage_levels::AddXpResult;
use serde::Deserialize;
use serde::Serialize;
// ── Request DTOs ──

#[derive(Debug, Deserialize)]
pub struct AddXpDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub amount: i64,
    /// "text" ou "voice" (defaut: "text")
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "text".to_string()
}

/// Set la valeur exacte XP texte/voix d'un user (admin override).
/// Champs Option : non envoye = on ne touche pas a ce champ.
#[derive(Debug, Deserialize)]
pub struct SetUserXpDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub xp_text: Option<i64>,
    pub xp_voice: Option<i64>,
}

/// Reset XP d'un user (admin override).
/// `target` = "all" / "text" / "voice".
#[derive(Debug, Deserialize)]
pub struct ResetUserXpDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub target: String,
}

#[derive(Debug, Deserialize)]
pub struct LevelLeaderboardParams {
    pub limit: Option<i64>,
    /// "text", "voice" ou absent (= total)
    pub source: Option<String>,
}

// ── Response DTOs ──

#[derive(Debug, Serialize)]
pub struct UserLevelDto {
    pub id: String,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub xp: i64,
    pub level: i32,
    pub xp_current: i64,
    pub xp_needed: i64,
    pub xp_text: i64,
    pub level_text: i32,
    pub xp_text_current: i64,
    pub xp_text_needed: i64,
    pub xp_voice: i64,
    pub level_voice: i32,
    pub xp_voice_current: i64,
    pub xp_voice_needed: i64,
    pub last_xp_at: String,
}

#[derive(Debug, Serialize)]
pub struct AddXpResponseDto {
    pub user: UserLevelDto,
    pub leveled_up: bool,
    pub old_level: i32,
    pub old_level_global: i32,
    pub source: String,
}

// ── From impls ──

impl From<UserLevel> for UserLevelDto {
    fn from(u: UserLevel) -> Self {
        let (xp_current, xp_needed) = xp_progress(u.xp);
        let (xp_text_current, xp_text_needed) = xp_progress(u.xp_text);
        let (xp_voice_current, xp_voice_needed) = xp_progress(u.xp_voice);
        Self {
            id: u.id.to_string(),
            guild_id: u.guild_id,
            user_id: u.user_id,
            username: u.username,
            xp: u.xp,
            level: u.level,
            xp_current,
            xp_needed,
            xp_text: u.xp_text,
            level_text: u.level_text,
            xp_text_current,
            xp_text_needed,
            xp_voice: u.xp_voice,
            level_voice: u.level_voice,
            xp_voice_current,
            xp_voice_needed,
            last_xp_at: u.last_xp_at.to_rfc3339(),
        }
    }
}

impl From<AddXpResult> for AddXpResponseDto {
    fn from(r: AddXpResult) -> Self {
        Self {
            user: UserLevelDto::from(r.user_level),
            leveled_up: r.leveled_up,
            old_level: r.old_level,
            old_level_global: r.old_level_global,
            source: r.source.as_str().to_string(),
        }
    }
}

#[cfg(test)]
#[path = "tests/levels.rs"]
mod tests;
