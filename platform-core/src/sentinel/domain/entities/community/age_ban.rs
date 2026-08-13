//! Ban temporaire issu de la verification d'age au reglement.
//!
//! Quand un membre declare un age inferieur au minimum requis, il est banni
//! jusqu'a ce qu'il atteigne cet age. La source de verite de la date de deban
//! est `unban_at` ; un job worker mensuel debannit les `pending` echus.

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgeBanStatus {
    Pending,
    Lifted,
}

impl AgeBanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgeBanStatus::Pending => "pending",
            AgeBanStatus::Lifted => "lifted",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "lifted" => AgeBanStatus::Lifted,
            _ => AgeBanStatus::Pending,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgeBan {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub declared_age: i32,
    pub banned_at: DateTime<Utc>,
    pub unban_at: DateTime<Utc>,
    pub status: AgeBanStatus,
    pub lifted_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
#[path = "tests/age_ban.rs"]
mod tests;
