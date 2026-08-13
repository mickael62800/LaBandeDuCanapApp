use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::announcement::{
    AnnouncementButton, AnnouncementRun, ButtonInteraction, ChannelPostResult, ContentType,
    RecurrenceType, RunStatus, ScheduledAnnouncement,
};

#[derive(Debug, Deserialize)]
pub struct CreateAnnouncementDto {
    pub guild_id: String,
    pub name: String,
    pub recurrence_type: String,
    pub recurrence_hour: u8,
    #[serde(default)]
    pub recurrence_minute: u8,
    pub recurrence_day_of_week: Option<u8>,
    pub recurrence_day_of_month: Option<u8>,
    #[serde(default)]
    pub recurrence_month: Option<u8>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub content_type: String,
    #[serde(default)]
    pub content_text: String,
    pub embed_title: Option<String>,
    pub embed_color: Option<i32>,
    pub embed_image_url: Option<String>,
    pub embed_thumbnail_url: Option<String>,
    pub embed_footer_text: Option<String>,
    #[serde(default)]
    pub mention_everyone: bool,
    #[serde(default)]
    pub mention_here: bool,
    #[serde(default)]
    pub mention_role_ids: Vec<String>,
    pub channel_ids: Vec<String>,
    #[serde(default)]
    pub buttons: Vec<AnnouncementButton>,
    #[serde(default)]
    pub auto_reactions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAnnouncementDto {
    pub name: String,
    pub recurrence_type: String,
    pub recurrence_hour: u8,
    #[serde(default)]
    pub recurrence_minute: u8,
    pub recurrence_day_of_week: Option<u8>,
    pub recurrence_day_of_month: Option<u8>,
    #[serde(default)]
    pub recurrence_month: Option<u8>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub content_type: String,
    #[serde(default)]
    pub content_text: String,
    pub embed_title: Option<String>,
    pub embed_color: Option<i32>,
    pub embed_image_url: Option<String>,
    pub embed_thumbnail_url: Option<String>,
    pub embed_footer_text: Option<String>,
    #[serde(default)]
    pub mention_everyone: bool,
    #[serde(default)]
    pub mention_here: bool,
    #[serde(default)]
    pub mention_role_ids: Vec<String>,
    pub channel_ids: Vec<String>,
    #[serde(default)]
    pub buttons: Vec<AnnouncementButton>,
    #[serde(default)]
    pub auto_reactions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ToggleAnnouncementDto {
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct AnnouncementDto {
    pub id: Uuid,
    pub guild_id: String,
    pub name: String,
    pub enabled: bool,
    pub recurrence_type: String,
    pub recurrence_hour: u8,
    pub recurrence_minute: u8,
    pub recurrence_day_of_week: Option<u8>,
    pub recurrence_day_of_month: Option<u8>,
    pub recurrence_month: Option<u8>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub content_type: String,
    pub content_text: String,
    pub embed_title: Option<String>,
    pub embed_color: Option<i32>,
    pub embed_image_url: Option<String>,
    pub embed_thumbnail_url: Option<String>,
    pub embed_footer_text: Option<String>,
    pub mention_everyone: bool,
    pub mention_here: bool,
    pub mention_role_ids: Vec<String>,
    pub channel_ids: Vec<String>,
    pub buttons: Vec<AnnouncementButton>,
    pub auto_reactions: Vec<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: DateTime<Utc>,
}

impl From<ScheduledAnnouncement> for AnnouncementDto {
    fn from(a: ScheduledAnnouncement) -> Self {
        Self {
            id: a.id,
            guild_id: a.guild_id,
            name: a.name,
            enabled: a.enabled,
            recurrence_type: a.recurrence_type.as_str().to_string(),
            recurrence_hour: a.recurrence_hour,
            recurrence_minute: a.recurrence_minute,
            recurrence_day_of_week: a.recurrence_day_of_week,
            recurrence_day_of_month: a.recurrence_day_of_month,
            recurrence_month: a.recurrence_month,
            scheduled_at: a.scheduled_at,
            start_date: a.start_date,
            end_date: a.end_date,
            content_type: a.content_type.as_str().to_string(),
            content_text: a.content_text,
            embed_title: a.embed_title,
            embed_color: a.embed_color,
            embed_image_url: a.embed_image_url,
            embed_thumbnail_url: a.embed_thumbnail_url,
            embed_footer_text: a.embed_footer_text,
            mention_everyone: a.mention_everyone,
            mention_here: a.mention_here,
            mention_role_ids: a.mention_role_ids,
            channel_ids: a.channel_ids,
            buttons: a.buttons,
            auto_reactions: a.auto_reactions,
            created_by: a.created_by,
            created_at: a.created_at,
            updated_at: a.updated_at,
            last_run_at: a.last_run_at,
            next_run_at: a.next_run_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AnnouncementRunDto {
    pub id: Uuid,
    pub announcement_id: Uuid,
    pub guild_id: String,
    pub ran_at: DateTime<Utc>,
    pub channels_posted: Vec<ChannelPostResult>,
    pub status: String,
    pub error: Option<String>,
}

impl From<AnnouncementRun> for AnnouncementRunDto {
    fn from(r: AnnouncementRun) -> Self {
        Self {
            id: r.id,
            announcement_id: r.announcement_id,
            guild_id: r.guild_id,
            ran_at: r.ran_at,
            channels_posted: r.channels_posted,
            status: r.status.as_str().to_string(),
            error: r.error,
        }
    }
}

// ── Helpers conversion enum ────────────────────────────────────────────

pub fn parse_recurrence(s: &str) -> Result<RecurrenceType, String> {
    RecurrenceType::from_str(s).ok_or_else(|| {
        format!(
            "recurrence_type invalide: {} (attendu once/daily/weekly/monthly/yearly)",
            s
        )
    })
}

pub fn parse_content_type(s: &str) -> Result<ContentType, String> {
    ContentType::from_str(s)
        .ok_or_else(|| format!("content_type invalide: {} (attendu text/embed)", s))
}

pub fn parse_run_status(s: &str) -> Result<RunStatus, String> {
    RunStatus::from_str(s).ok_or_else(|| format!("status invalide: {}", s))
}

// ── Worker endpoints (interne) ─────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordRunResultDto {
    pub channels_posted: Vec<ChannelPostResult>,
}

#[derive(Debug, Deserialize)]
pub struct ButtonClickDto {
    pub announcement_id: Uuid,
    pub run_id: Option<Uuid>,
    pub user_id: String,
    pub user_name: Option<String>,
    pub button_custom_id: String,
    pub button_label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ButtonInteractionDto {
    pub id: Uuid,
    pub announcement_id: Uuid,
    pub run_id: Option<Uuid>,
    pub user_id: String,
    pub user_name: Option<String>,
    pub button_custom_id: String,
    pub button_label: Option<String>,
    pub clicked_at: DateTime<Utc>,
}

impl From<ButtonInteraction> for ButtonInteractionDto {
    fn from(b: ButtonInteraction) -> Self {
        Self {
            id: b.id,
            announcement_id: b.announcement_id,
            run_id: b.run_id,
            user_id: b.user_id,
            user_name: b.user_name,
            button_custom_id: b.button_custom_id,
            button_label: b.button_label,
            clicked_at: b.clicked_at,
        }
    }
}
