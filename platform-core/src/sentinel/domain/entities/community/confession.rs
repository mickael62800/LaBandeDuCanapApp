//! Entites domain pour le systeme de confessions anonymes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Confession {
    pub id: Uuid,
    pub guild_id: String,
    pub public_number: i32,
    pub author_user_id: String,
    pub content: String,
    pub message_id: Option<String>,
    pub channel_id: Option<String>,
    pub thread_id: Option<String>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<String>,
    pub deleted_reason: Option<String>,
    pub edited_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ConfessionReply {
    pub id: Uuid,
    pub confession_id: Uuid,
    pub public_number: i32,
    pub author_user_id: String,
    pub content: String,
    pub is_anonymous: bool,
    pub message_id: Option<String>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<String>,
    pub edited_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Pending,
    Resolved,
    Dismissed,
}

impl ReportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Resolved => "resolved",
            Self::Dismissed => "dismissed",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "resolved" => Some(Self::Resolved),
            "dismissed" => Some(Self::Dismissed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfessionReport {
    pub id: Uuid,
    pub guild_id: String,
    pub confession_id: Option<Uuid>,
    pub reply_id: Option<Uuid>,
    pub reporter_user_id: String,
    pub reason: String,
    pub status: ReportStatus,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ConfessionConfig {
    pub guild_id: String,
    pub enabled: bool,
    pub channel_id: Option<String>,
    pub panel_message_id: Option<String>,
    pub cooldown_secs: i32,
    pub max_per_day: i32,
    /// Fenetre glissante (en heures) sur laquelle `max_per_day` est compte.
    /// Defaut 24h. Bornee a >= 1 a l'usage.
    pub quota_window_hours: i32,
    pub min_chars: i32,
    pub max_chars: i32,
    pub automod_enabled: bool,
    pub banned_user_ids: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

impl ConfessionConfig {
    pub fn defaults(guild_id: String) -> Self {
        Self {
            guild_id,
            enabled: true,
            channel_id: None,
            panel_message_id: None,
            cooldown_secs: 60,
            max_per_day: 20,
            quota_window_hours: 24,
            min_chars: 5,
            max_chars: 2000,
            automod_enabled: true,
            banned_user_ids: Vec::new(),
            updated_at: Utc::now(),
        }
    }

    /// Verifie qu'un contenu respecte les bornes de la config.
    /// Retourne Err(message) si invalide.
    pub fn validate_content(&self, content: &str) -> Result<(), String> {
        let trimmed = content.trim();
        let len = trimmed.chars().count() as i32;
        if len < self.min_chars {
            return Err(format!(
                "Confession trop courte ({} caracteres min, {} fourni)",
                self.min_chars, len
            ));
        }
        if len > self.max_chars {
            return Err(format!(
                "Confession trop longue ({} caracteres max, {} fourni)",
                self.max_chars, len
            ));
        }
        Ok(())
    }

    pub fn is_user_banned(&self, user_id: &str) -> bool {
        self.banned_user_ids.iter().any(|u| u == user_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_content_min_chars() {
        let cfg = ConfessionConfig::defaults("g1".into());
        assert!(cfg.validate_content("hi").is_err());
        assert!(cfg.validate_content("hello").is_ok());
    }

    #[test]
    fn validate_content_max_chars() {
        let cfg = ConfessionConfig::defaults("g1".into());
        let long = "x".repeat(2001);
        assert!(cfg.validate_content(&long).is_err());
        let ok = "x".repeat(2000);
        assert!(cfg.validate_content(&ok).is_ok());
    }

    #[test]
    fn is_user_banned() {
        let mut cfg = ConfessionConfig::defaults("g1".into());
        cfg.banned_user_ids.push("123".into());
        assert!(cfg.is_user_banned("123"));
        assert!(!cfg.is_user_banned("456"));
    }
}
