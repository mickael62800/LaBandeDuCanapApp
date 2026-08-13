use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordRole {
    pub id: String,
    pub guild_id: GuildId,
    pub name: String,
    pub color: i32,
    pub position: i32,
    pub permissions: i64,
    pub mentionable: bool,
    pub managed: bool,
    pub icon: Option<String>,
    pub member_count: i32,
    pub synced_at: DateTime<Utc>,
}

/// Parse un bitfield de permissions Discord (string) en `i64`. Fallback 0
/// si l'input est invalide ou vide. Regle metier : les permissions Discord
/// sont des BigInt en JSON (depassent Number.MAX_SAFE_INTEGER), on stocke
/// en bigint cote DB / i64 cote Rust.
pub fn parse_discord_permissions_bitfield(s: &str) -> i64 {
    s.parse::<i64>().unwrap_or(0)
}

#[cfg(test)]
#[path = "tests/discord_role.rs"]
mod tests;
