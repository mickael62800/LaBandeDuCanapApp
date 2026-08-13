use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use chrono::DateTime;
use chrono::Utc;
#[derive(Debug, Clone)]
pub struct WatchedUser {
    pub user_id: UserId,
    pub username: String,
    pub guild_id: GuildId,
    pub guild_name: String,
    pub risk_level: String,
    pub total_warns: i64,
    pub total_mutes: i64,
    pub total_bans: i64,
    pub last_incident_at: Option<DateTime<Utc>>,
    pub security_events_count: i64,
    pub first_seen_at: DateTime<Utc>,
}

/// Classification de risque d'un utilisateur surveille selon ses compteurs
/// d'infractions. Regle metier pure (pas d'I/O).
///
/// Seuils :
/// - `critical` : au moins 1 ban OU total >= 5
/// - `high`     : au moins 1 mute OU total >= 3
/// - `medium`   : total >= 2
/// - `low`      : sinon
pub fn classify_risk_level(total_warns: i64, total_mutes: i64, total_bans: i64) -> &'static str {
    let total = total_warns + total_mutes + total_bans;
    if total_bans > 0 || total >= 5 {
        "critical"
    } else if total_mutes > 0 || total >= 3 {
        "high"
    } else if total >= 2 {
        "medium"
    } else {
        "low"
    }
}

#[cfg(test)]
#[path = "tests/watched_user.rs"]
mod tests;
