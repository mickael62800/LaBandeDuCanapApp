use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;

use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelPreset;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::voice_channel_repository::VoicePresetStore;

#[derive(sqlx::FromRow)]
struct PresetRow {
    guild_id: String,
    owner_id: String,
    channel_name: Option<String>,
    member_limit: Option<i32>,
    visibility: String,
    locked: bool,
    queue_enabled: bool,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PresetRow> for VoiceChannelPreset {
    fn from(row: PresetRow) -> Self {
        Self {
            guild_id: row.guild_id.into(),
            owner_id: row.owner_id,
            channel_name: row.channel_name,
            member_limit: row.member_limit,
            visibility: row.visibility,
            locked: row.locked,
            queue_enabled: row.queue_enabled,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl VoicePresetStore for super::PgVoiceChannelRepository {
    async fn find_preset(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<Option<VoiceChannelPreset>, DomainError> {
        let row = sqlx::query_as::<_, PresetRow>(
            "SELECT * FROM voice_channel_presets WHERE guild_id = $1 AND owner_id = $2",
        )
        .bind(guild_id)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.map(VoiceChannelPreset::from))
    }

    async fn upsert_preset(&self, preset: &VoiceChannelPreset) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO voice_channel_presets (guild_id, owner_id, channel_name, member_limit, visibility, locked, queue_enabled, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (guild_id, owner_id) DO UPDATE SET
                channel_name = EXCLUDED.channel_name,
                member_limit = EXCLUDED.member_limit,
                visibility = EXCLUDED.visibility,
                locked = EXCLUDED.locked,
                queue_enabled = EXCLUDED.queue_enabled,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(preset.guild_id.as_str())
        .bind(preset.owner_id.as_str())
        .bind(&preset.channel_name)
        .bind(preset.member_limit)
        .bind(&preset.visibility)
        .bind(preset.locked)
        .bind(preset.queue_enabled)
        .bind(preset.updated_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }
}
