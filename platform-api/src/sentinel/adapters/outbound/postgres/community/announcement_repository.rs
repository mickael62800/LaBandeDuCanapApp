use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::announcement::{
    AnnouncementRun, ButtonInteraction, ChannelPostResult, ContentType, RecurrenceType, RunStatus,
    ScheduledAnnouncement,
};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::announcement_repository::AnnouncementRepository;

pub struct PgAnnouncementRepository {
    pool: PgPool,
}

impl PgAnnouncementRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct AnnouncementRow {
    id: Uuid,
    guild_id: String,
    name: String,
    enabled: bool,
    recurrence_type: String,
    recurrence_hour: i16,
    recurrence_minute: i16,
    recurrence_day_of_week: Option<i16>,
    recurrence_day_of_month: Option<i16>,
    recurrence_month: Option<i16>,
    scheduled_at: Option<DateTime<Utc>>,
    start_date: DateTime<Utc>,
    end_date: Option<DateTime<Utc>>,
    content_type: String,
    content_text: String,
    embed_title: Option<String>,
    embed_color: Option<i32>,
    embed_image_url: Option<String>,
    embed_thumbnail_url: Option<String>,
    embed_footer_text: Option<String>,
    mention_everyone: bool,
    mention_here: bool,
    mention_role_ids: serde_json::Value,
    channel_ids: serde_json::Value,
    buttons: serde_json::Value,
    auto_reactions: serde_json::Value,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_run_at: Option<DateTime<Utc>>,
    next_run_at: DateTime<Utc>,
}

impl From<AnnouncementRow> for ScheduledAnnouncement {
    fn from(r: AnnouncementRow) -> Self {
        let role_ids: Vec<String> = serde_json::from_value(r.mention_role_ids).unwrap_or_default();
        let chans: Vec<String> = serde_json::from_value(r.channel_ids).unwrap_or_default();
        Self {
            id: r.id,
            guild_id: r.guild_id,
            name: r.name,
            enabled: r.enabled,
            recurrence_type: RecurrenceType::from_str(&r.recurrence_type)
                .unwrap_or(RecurrenceType::Daily),
            recurrence_hour: r.recurrence_hour as u8,
            recurrence_minute: r.recurrence_minute as u8,
            recurrence_day_of_week: r.recurrence_day_of_week.map(|v| v as u8),
            recurrence_day_of_month: r.recurrence_day_of_month.map(|v| v as u8),
            recurrence_month: r.recurrence_month.map(|v| v as u8),
            scheduled_at: r.scheduled_at,
            start_date: r.start_date,
            end_date: r.end_date,
            content_type: ContentType::from_str(&r.content_type).unwrap_or(ContentType::Text),
            content_text: r.content_text,
            embed_title: r.embed_title,
            embed_color: r.embed_color,
            embed_image_url: r.embed_image_url,
            embed_thumbnail_url: r.embed_thumbnail_url,
            embed_footer_text: r.embed_footer_text,
            mention_everyone: r.mention_everyone,
            mention_here: r.mention_here,
            mention_role_ids: role_ids,
            channel_ids: chans,
            buttons: serde_json::from_value(r.buttons).unwrap_or_default(),
            auto_reactions: serde_json::from_value(r.auto_reactions).unwrap_or_default(),
            created_by: r.created_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
            last_run_at: r.last_run_at,
            next_run_at: r.next_run_at,
        }
    }
}

#[derive(FromRow)]
struct RunRow {
    id: Uuid,
    announcement_id: Uuid,
    guild_id: String,
    ran_at: DateTime<Utc>,
    channels_posted: serde_json::Value,
    status: String,
    error: Option<String>,
}

impl From<RunRow> for AnnouncementRun {
    fn from(r: RunRow) -> Self {
        let channels: Vec<ChannelPostResult> =
            serde_json::from_value(r.channels_posted).unwrap_or_default();
        Self {
            id: r.id,
            announcement_id: r.announcement_id,
            guild_id: r.guild_id,
            ran_at: r.ran_at,
            channels_posted: channels,
            status: RunStatus::from_str(&r.status).unwrap_or(RunStatus::Error),
            error: r.error,
        }
    }
}

const SELECT_ANNOUNCEMENT: &str = r#"
    SELECT id, guild_id, name, enabled,
        recurrence_type, recurrence_hour, recurrence_minute,
        recurrence_day_of_week, recurrence_day_of_month, recurrence_month, scheduled_at,
        start_date, end_date,
        content_type, content_text, embed_title, embed_color,
        embed_image_url, embed_thumbnail_url, embed_footer_text,
        mention_everyone, mention_here, mention_role_ids,
        channel_ids, buttons, auto_reactions,
        created_by, created_at, updated_at,
        last_run_at, next_run_at
    FROM scheduled_announcements
"#;

#[async_trait]
impl AnnouncementRepository for PgAnnouncementRepository {
    async fn create(&self, a: &ScheduledAnnouncement) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO scheduled_announcements (
                id, guild_id, name, enabled,
                recurrence_type, recurrence_hour, recurrence_minute,
                recurrence_day_of_week, recurrence_day_of_month, recurrence_month, scheduled_at,
                start_date, end_date,
                content_type, content_text, embed_title, embed_color,
                embed_image_url, embed_thumbnail_url, embed_footer_text,
                mention_everyone, mention_here, mention_role_ids,
                channel_ids, buttons, auto_reactions,
                created_by, created_at, updated_at,
                last_run_at, next_run_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                $13, $14, $15, $16, $17, $18, $19, $20, $21, $22,
                $23, $24, $25, $26, $27, $28, $29, $30, $31
            )"#,
        )
        .bind(a.id)
        .bind(a.guild_id.as_str())
        .bind(&a.name)
        .bind(a.enabled)
        .bind(a.recurrence_type.as_str())
        .bind(a.recurrence_hour as i16)
        .bind(a.recurrence_minute as i16)
        .bind(a.recurrence_day_of_week.map(|v| v as i16))
        .bind(a.recurrence_day_of_month.map(|v| v as i16))
        .bind(a.recurrence_month.map(|v| v as i16))
        .bind(a.scheduled_at)
        .bind(a.start_date)
        .bind(a.end_date)
        .bind(a.content_type.as_str())
        .bind(&a.content_text)
        .bind(&a.embed_title)
        .bind(a.embed_color)
        .bind(&a.embed_image_url)
        .bind(&a.embed_thumbnail_url)
        .bind(&a.embed_footer_text)
        .bind(a.mention_everyone)
        .bind(a.mention_here)
        .bind(serde_json::to_value(&a.mention_role_ids).unwrap_or_default())
        .bind(serde_json::to_value(&a.channel_ids).unwrap_or_default())
        .bind(serde_json::to_value(&a.buttons).unwrap_or_default())
        .bind(serde_json::to_value(&a.auto_reactions).unwrap_or_default())
        .bind(a.created_by.as_str())
        .bind(a.created_at)
        .bind(a.updated_at)
        .bind(a.last_run_at)
        .bind(a.next_run_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn update(&self, a: &ScheduledAnnouncement) -> Result<(), DomainError> {
        sqlx::query(
            r#"UPDATE scheduled_announcements SET
                name = $2, enabled = $3,
                recurrence_type = $4, recurrence_hour = $5, recurrence_minute = $6,
                recurrence_day_of_week = $7, recurrence_day_of_month = $8, scheduled_at = $9,
                start_date = $10, end_date = $11,
                content_type = $12, content_text = $13, embed_title = $14, embed_color = $15,
                embed_image_url = $16, embed_thumbnail_url = $17,
                mention_everyone = $18, mention_here = $19, mention_role_ids = $20,
                channel_ids = $21, buttons = $22, auto_reactions = $23,
                recurrence_month = $25, embed_footer_text = $26,
                updated_at = NOW(), next_run_at = $24
            WHERE id = $1"#,
        )
        .bind(a.id)
        .bind(&a.name)
        .bind(a.enabled)
        .bind(a.recurrence_type.as_str())
        .bind(a.recurrence_hour as i16)
        .bind(a.recurrence_minute as i16)
        .bind(a.recurrence_day_of_week.map(|v| v as i16))
        .bind(a.recurrence_day_of_month.map(|v| v as i16))
        .bind(a.scheduled_at)
        .bind(a.start_date)
        .bind(a.end_date)
        .bind(a.content_type.as_str())
        .bind(&a.content_text)
        .bind(&a.embed_title)
        .bind(a.embed_color)
        .bind(&a.embed_image_url)
        .bind(&a.embed_thumbnail_url)
        .bind(&a.embed_footer_text)
        .bind(a.mention_everyone)
        .bind(a.mention_here)
        .bind(serde_json::to_value(&a.mention_role_ids).unwrap_or_default())
        .bind(serde_json::to_value(&a.channel_ids).unwrap_or_default())
        .bind(serde_json::to_value(&a.buttons).unwrap_or_default())
        .bind(serde_json::to_value(&a.auto_reactions).unwrap_or_default())
        .bind(a.next_run_at)
        .bind(a.recurrence_month.map(|v| v as i16))
        .bind(&a.embed_footer_text)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM scheduled_announcements WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<ScheduledAnnouncement>, DomainError> {
        let q = format!("{} WHERE id = $1", SELECT_ANNOUNCEMENT);
        let row = sqlx::query_as::<_, AnnouncementRow>(&q)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(ScheduledAnnouncement::from))
    }

    async fn list_by_guild(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ScheduledAnnouncement>, DomainError> {
        let q = format!(
            "{} WHERE guild_id = $1 ORDER BY created_at DESC",
            SELECT_ANNOUNCEMENT
        );
        let rows = sqlx::query_as::<_, AnnouncementRow>(&q)
            .bind(guild_id)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(ScheduledAnnouncement::from).collect())
    }

    async fn set_enabled(&self, id: Uuid, enabled: bool) -> Result<bool, DomainError> {
        sqlx::query(
            "UPDATE scheduled_announcements SET enabled = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(enabled)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(enabled)
    }

    async fn list_due(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<ScheduledAnnouncement>, DomainError> {
        let q = format!(
            "{} WHERE enabled = TRUE AND next_run_at <= $1 ORDER BY next_run_at ASC LIMIT $2",
            SELECT_ANNOUNCEMENT
        );
        let rows = sqlx::query_as::<_, AnnouncementRow>(&q)
            .bind(now)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(ScheduledAnnouncement::from).collect())
    }

    async fn mark_run(
        &self,
        id: Uuid,
        last_run_at: DateTime<Utc>,
        next_run_at: Option<DateTime<Utc>>,
    ) -> Result<(), DomainError> {
        match next_run_at {
            Some(nr) => {
                sqlx::query(
                    "UPDATE scheduled_announcements SET last_run_at = $2, next_run_at = $3, updated_at = NOW() WHERE id = $1",
                )
                .bind(id)
                .bind(last_run_at)
                .bind(nr)
                .execute(&self.pool)
                .await
                .map_err(pg_err)?;
            }
            None => {
                // Annonce terminee : on disable + on garde un next_run_at
                // dans un futur lointain pour respecter la NOT NULL constraint.
                let far_future = chrono::Utc::now() + chrono::Duration::days(365 * 100);
                sqlx::query(
                    "UPDATE scheduled_announcements SET last_run_at = $2, enabled = FALSE, next_run_at = $3, updated_at = NOW() WHERE id = $1",
                )
                .bind(id)
                .bind(last_run_at)
                .bind(far_future)
                .execute(&self.pool)
                .await
                .map_err(pg_err)?;
            }
        }
        Ok(())
    }

    async fn insert_run(&self, run: &AnnouncementRun) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO scheduled_announcement_runs
                (id, announcement_id, guild_id, ran_at, channels_posted, status, error)
                VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        )
        .bind(run.id)
        .bind(run.announcement_id)
        .bind(run.guild_id.as_str())
        .bind(run.ran_at)
        .bind(serde_json::to_value(&run.channels_posted).unwrap_or_default())
        .bind(run.status.as_str())
        .bind(&run.error)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn update_run_result(
        &self,
        run_id: Uuid,
        status: RunStatus,
        channels_posted: &[ChannelPostResult],
        error: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE scheduled_announcement_runs SET status = $2, channels_posted = $3, error = $4 WHERE id = $1",
        )
        .bind(run_id)
        .bind(status.as_str())
        .bind(serde_json::to_value(channels_posted).unwrap_or_default())
        .bind(error)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_runs(
        &self,
        announcement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AnnouncementRun>, DomainError> {
        let rows = sqlx::query_as::<_, RunRow>(
            "SELECT id, announcement_id, guild_id, ran_at, channels_posted, status, error
             FROM scheduled_announcement_runs
             WHERE announcement_id = $1 ORDER BY ran_at DESC LIMIT $2",
        )
        .bind(announcement_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(AnnouncementRun::from).collect())
    }

    async fn record_button_interaction(&self, i: &ButtonInteraction) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO announcement_button_interactions
                (id, announcement_id, run_id, user_id, user_name, button_custom_id, button_label, clicked_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(i.id)
        .bind(i.announcement_id)
        .bind(i.run_id)
        .bind(i.user_id.as_str())
        .bind(&i.user_name)
        .bind(&i.button_custom_id)
        .bind(&i.button_label)
        .bind(i.clicked_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_button_interactions(
        &self,
        announcement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ButtonInteraction>, DomainError> {
        #[derive(sqlx::FromRow)]
        struct InterRow {
            id: Uuid,
            announcement_id: Uuid,
            run_id: Option<Uuid>,
            user_id: String,
            user_name: Option<String>,
            button_custom_id: String,
            button_label: Option<String>,
            clicked_at: chrono::DateTime<chrono::Utc>,
        }
        let rows = sqlx::query_as::<_, InterRow>(
            "SELECT id, announcement_id, run_id, user_id, user_name, button_custom_id, button_label, clicked_at
             FROM announcement_button_interactions
             WHERE announcement_id = $1 ORDER BY clicked_at DESC LIMIT $2",
        )
        .bind(announcement_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|r| ButtonInteraction {
                id: r.id,
                announcement_id: r.announcement_id,
                run_id: r.run_id,
                user_id: r.user_id,
                user_name: r.user_name,
                button_custom_id: r.button_custom_id,
                button_label: r.button_label,
                clicked_at: r.clicked_at,
            })
            .collect())
    }

    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT guild_id FROM guilds ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(|(g,)| g).collect())
    }

    async fn delete_runs_older_than(&self, guild_id: &str, days: i32) -> Result<u64, DomainError> {
        let res = sqlx::query(
            "DELETE FROM scheduled_announcement_runs WHERE guild_id = $1 AND ran_at < NOW() - ($2::int * INTERVAL '1 day')",
        )
        .bind(guild_id)
        .bind(days)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(res.rows_affected())
    }
}
