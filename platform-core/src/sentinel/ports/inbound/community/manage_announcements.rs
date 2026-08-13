use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::sentinel::domain::entities::community::announcement::{
    AnnouncementButton, AnnouncementRun, ButtonInteraction, ChannelPostResult, ContentType,
    RecurrenceType, ScheduledAnnouncement,
};
use crate::sentinel::domain::errors::DomainError;

pub struct CreateAnnouncementCommand {
    pub guild_id: String,
    pub name: String,
    pub recurrence_type: RecurrenceType,
    pub recurrence_hour: u8,
    pub recurrence_minute: u8,
    pub recurrence_day_of_week: Option<u8>,
    pub recurrence_day_of_month: Option<u8>,
    pub recurrence_month: Option<u8>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub content_type: ContentType,
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
}

pub struct UpdateAnnouncementCommand {
    pub id: Uuid,
    pub name: String,
    pub recurrence_type: RecurrenceType,
    pub recurrence_hour: u8,
    pub recurrence_minute: u8,
    pub recurrence_day_of_week: Option<u8>,
    pub recurrence_day_of_month: Option<u8>,
    pub recurrence_month: Option<u8>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub content_type: ContentType,
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
}

/// Payload pret a etre envoye au bot Discord pour publication.
/// Variables deja interpolees, mentions formattees.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RenderedAnnouncement {
    pub announcement_id: Uuid,
    pub run_id: Uuid,
    pub guild_id: String,
    pub channel_ids: Vec<String>,
    pub content_text: String,
    pub embed: Option<RenderedEmbed>,
    /// Texte de mentions a prepend au message (ex: "@everyone <@&role_id>").
    pub mentions_prefix: String,
    pub buttons: Vec<AnnouncementButton>,
    pub auto_reactions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RenderedEmbed {
    pub title: Option<String>,
    pub description: String,
    pub color: Option<i32>,
    pub image_url: Option<String>,
    pub thumbnail_url: Option<String>,
    /// Rendu sous l'image par Discord.
    #[serde(default)]
    pub footer_text: Option<String>,
}

/// Resultat d'une passe de purge de l'historique (`announcement_runs`)
/// sur l'ensemble des guilds.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RetentionCleanupSummary {
    pub guilds_processed: u64,
    pub guilds_skipped: u64,
    pub rows_deleted: i64,
}

#[async_trait]
pub trait ManageAnnouncementsUseCase: Send + Sync {
    async fn create(
        &self,
        cmd: CreateAnnouncementCommand,
    ) -> Result<ScheduledAnnouncement, DomainError>;
    async fn update(
        &self,
        cmd: UpdateAnnouncementCommand,
    ) -> Result<ScheduledAnnouncement, DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    async fn get(&self, id: Uuid) -> Result<ScheduledAnnouncement, DomainError>;
    async fn list_by_guild(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ScheduledAnnouncement>, DomainError>;
    async fn toggle(&self, id: Uuid, enabled: bool) -> Result<bool, DomainError>;

    /// Pour les workers : recupere les annonces dues + cree un run pending +
    /// retourne les payloads prets a publier (variables interpolees).
    async fn fetch_due_and_prepare(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<RenderedAnnouncement>, DomainError>;

    /// Apres publication par le bot : enregistre le resultat de chaque
    /// channel + calcule next_run_at.
    async fn record_run_result(
        &self,
        run_id: Uuid,
        channels_posted: Vec<ChannelPostResult>,
    ) -> Result<(), DomainError>;

    /// Aperçu : retourne le rendu sans poster ni creer de run.
    async fn preview(&self, id: Uuid) -> Result<RenderedAnnouncement, DomainError>;

    async fn list_runs(
        &self,
        announcement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AnnouncementRun>, DomainError>;

    /// Enregistre un clic sur un bouton (appele par le bot apres
    /// interaction Discord).
    async fn record_button_interaction(
        &self,
        announcement_id: Uuid,
        run_id: Option<Uuid>,
        user_id: String,
        user_name: Option<String>,
        button_custom_id: String,
        button_label: Option<String>,
    ) -> Result<(), DomainError>;

    async fn list_button_interactions(
        &self,
        announcement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ButtonInteraction>, DomainError>;

    /// Purge l'historique des runs plus vieux que `history_retention_days`
    /// (defaut 90j) pour chaque guild dont le module annonces est actif.
    /// Une valeur <= 0 = illimite (guild skip).
    async fn retention_cleanup_all(&self) -> Result<RetentionCleanupSummary, DomainError>;
}
