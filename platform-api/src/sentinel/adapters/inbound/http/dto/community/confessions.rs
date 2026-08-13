use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::confession::{
    Confession, ConfessionConfig, ConfessionReply, ConfessionReport, ReportStatus,
};

fn default_quota_window_hours() -> i32 {
    24
}

// ── Request DTOs ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateConfessionDto {
    pub guild_id: String,
    pub author_user_id: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMessageRefsDto {
    pub message_id: String,
    pub channel_id: String,
    pub thread_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EditConfessionDto {
    pub author_user_id: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteConfessionDto {
    pub deleted_by: String,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateReplyDto {
    pub author_user_id: String,
    pub content: String,
    #[serde(default = "default_true")]
    pub is_anonymous: bool,
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct UpdateReplyMessageDto {
    pub message_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateReportDto {
    pub guild_id: String,
    pub confession_id: Option<Uuid>,
    pub reply_id: Option<Uuid>,
    pub reporter_user_id: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ResolveReportDto {
    pub status: String, // "resolved" | "dismissed"
    pub resolved_by: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveConfigDto {
    pub guild_id: String,
    pub enabled: bool,
    pub channel_id: Option<String>,
    pub panel_message_id: Option<String>,
    pub cooldown_secs: i32,
    pub max_per_day: i32,
    /// Fenetre glissante (heures) du quota `max_per_day`. Defaut 24 si omis.
    #[serde(default = "default_quota_window_hours")]
    pub quota_window_hours: i32,
    pub min_chars: i32,
    pub max_chars: i32,
    /// C1 : flag mort (aucun filtre de mots n'existe). Conserve pour
    /// back-compat mais plus surface ; `#[serde(default)]` => le bot peut
    /// l'omettre du body.
    #[serde(default)]
    pub automod_enabled: bool,
    #[serde(default)]
    pub banned_user_ids: Vec<String>,
}

// ── Response DTOs ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ConfessionDto {
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

impl From<Confession> for ConfessionDto {
    fn from(c: Confession) -> Self {
        Self {
            id: c.id,
            guild_id: c.guild_id,
            public_number: c.public_number,
            author_user_id: c.author_user_id,
            content: c.content,
            message_id: c.message_id,
            channel_id: c.channel_id,
            thread_id: c.thread_id,
            deleted_at: c.deleted_at,
            deleted_by: c.deleted_by,
            deleted_reason: c.deleted_reason,
            edited_at: c.edited_at,
            created_at: c.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ReplyDto {
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

impl From<ConfessionReply> for ReplyDto {
    fn from(r: ConfessionReply) -> Self {
        Self {
            id: r.id,
            confession_id: r.confession_id,
            public_number: r.public_number,
            author_user_id: r.author_user_id,
            content: r.content,
            is_anonymous: r.is_anonymous,
            message_id: r.message_id,
            deleted_at: r.deleted_at,
            deleted_by: r.deleted_by,
            edited_at: r.edited_at,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ReportDto {
    pub id: Uuid,
    pub guild_id: String,
    pub confession_id: Option<Uuid>,
    pub reply_id: Option<Uuid>,
    pub reporter_user_id: String,
    pub reason: String,
    pub status: String,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<ConfessionReport> for ReportDto {
    fn from(r: ConfessionReport) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            confession_id: r.confession_id,
            reply_id: r.reply_id,
            reporter_user_id: r.reporter_user_id,
            reason: r.reason,
            status: r.status.as_str().to_string(),
            resolved_by: r.resolved_by,
            resolved_at: r.resolved_at,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigDto {
    pub guild_id: String,
    pub enabled: bool,
    pub channel_id: Option<String>,
    pub panel_message_id: Option<String>,
    pub cooldown_secs: i32,
    pub max_per_day: i32,
    pub quota_window_hours: i32,
    pub min_chars: i32,
    pub max_chars: i32,
    pub automod_enabled: bool,
    pub banned_user_ids: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

impl From<ConfessionConfig> for ConfigDto {
    fn from(c: ConfessionConfig) -> Self {
        Self {
            guild_id: c.guild_id,
            enabled: c.enabled,
            channel_id: c.channel_id,
            panel_message_id: c.panel_message_id,
            cooldown_secs: c.cooldown_secs,
            max_per_day: c.max_per_day,
            quota_window_hours: c.quota_window_hours,
            min_chars: c.min_chars,
            max_chars: c.max_chars,
            automod_enabled: c.automod_enabled,
            banned_user_ids: c.banned_user_ids,
            updated_at: c.updated_at,
        }
    }
}

pub fn parse_report_status(s: &str) -> Result<ReportStatus, String> {
    ReportStatus::from_str(s).ok_or_else(|| format!("status invalide: {}", s))
}
