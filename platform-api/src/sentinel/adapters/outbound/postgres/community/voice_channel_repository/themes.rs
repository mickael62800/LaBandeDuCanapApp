use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelTheme;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::voice_channel_repository::VoiceThemeStore;

#[derive(sqlx::FromRow)]
struct ThemeRow {
    id: Uuid,
    guild_id: String,
    name: String,
    emoji: Option<String>,
    channel_name_template: String,
    member_limit: Option<i32>,
    visibility: String,
    locked: bool,
    queue_enabled: bool,
    bitrate: Option<i32>,
    slowmode_secs: Option<i32>,
    stage_enabled: bool,
    is_default: bool,
    sort_order: i32,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ThemeRow> for VoiceChannelTheme {
    fn from(row: ThemeRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id.into(),
            name: row.name,
            emoji: row.emoji,
            channel_name_template: row.channel_name_template,
            member_limit: row.member_limit,
            visibility: row.visibility,
            locked: row.locked,
            queue_enabled: row.queue_enabled,
            bitrate: row.bitrate,
            slowmode_secs: row.slowmode_secs,
            stage_enabled: row.stage_enabled,
            is_default: row.is_default,
            sort_order: row.sort_order,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl VoiceThemeStore for super::PgVoiceChannelRepository {
    async fn find_themes(&self, guild_id: &str) -> Result<Vec<VoiceChannelTheme>, DomainError> {
        let rows = sqlx::query_as::<_, ThemeRow>(
            "SELECT * FROM voice_channel_themes WHERE guild_id = $1 ORDER BY sort_order ASC, name ASC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(VoiceChannelTheme::from).collect())
    }

    async fn find_theme(&self, id: Uuid) -> Result<Option<VoiceChannelTheme>, DomainError> {
        let row = sqlx::query_as::<_, ThemeRow>("SELECT * FROM voice_channel_themes WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(row.map(VoiceChannelTheme::from))
    }

    async fn save_theme(&self, theme: &VoiceChannelTheme) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO voice_channel_themes (id, guild_id, name, emoji, channel_name_template, member_limit, visibility, locked, queue_enabled, bitrate, slowmode_secs, stage_enabled, is_default, sort_order, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            "#,
        )
        .bind(theme.id)
        .bind(theme.guild_id.as_str())
        .bind(&theme.name)
        .bind(&theme.emoji)
        .bind(&theme.channel_name_template)
        .bind(theme.member_limit)
        .bind(&theme.visibility)
        .bind(theme.locked)
        .bind(theme.queue_enabled)
        .bind(theme.bitrate)
        .bind(theme.slowmode_secs)
        .bind(theme.stage_enabled)
        .bind(theme.is_default)
        .bind(theme.sort_order)
        .bind(theme.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn update_theme(&self, theme: &VoiceChannelTheme) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE voice_channel_themes SET
                name = $2, emoji = $3, channel_name_template = $4, member_limit = $5,
                visibility = $6, locked = $7, queue_enabled = $8, bitrate = $9,
                slowmode_secs = $10, stage_enabled = $11, is_default = $12, sort_order = $13
            WHERE id = $1
            "#,
        )
        .bind(theme.id)
        .bind(&theme.name)
        .bind(&theme.emoji)
        .bind(&theme.channel_name_template)
        .bind(theme.member_limit)
        .bind(&theme.visibility)
        .bind(theme.locked)
        .bind(theme.queue_enabled)
        .bind(theme.bitrate)
        .bind(theme.slowmode_secs)
        .bind(theme.stage_enabled)
        .bind(theme.is_default)
        .bind(theme.sort_order)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn delete_theme(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM voice_channel_themes WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn clear_default_themes(&self, guild_id: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channel_themes SET is_default = false WHERE guild_id = $1 AND is_default = true")
            .bind(guild_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }
}
