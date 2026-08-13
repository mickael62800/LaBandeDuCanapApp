use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelWhitelistEntry;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::voice_channel_repository::VoiceWhitelistStore;

#[derive(sqlx::FromRow)]
struct WhitelistRow {
    id: Uuid,
    guild_id: String,
    owner_id: String,
    target_id: String,
    target_name: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<WhitelistRow> for VoiceChannelWhitelistEntry {
    fn from(row: WhitelistRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id.into(),
            owner_id: row.owner_id,
            target_id: row.target_id,
            target_name: row.target_name,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl VoiceWhitelistStore for super::PgVoiceChannelRepository {
    async fn find_whitelist(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError> {
        let rows = sqlx::query_as::<_, WhitelistRow>(
            "SELECT * FROM voice_channel_whitelists WHERE guild_id = $1 AND owner_id = $2 ORDER BY created_at ASC",
        )
        .bind(guild_id)
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(VoiceChannelWhitelistEntry::from)
            .collect())
    }

    async fn add_to_whitelist(
        &self,
        entry: &VoiceChannelWhitelistEntry,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO voice_channel_whitelists (id, guild_id, owner_id, target_id, target_name, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (guild_id, owner_id, target_id) DO NOTHING
            "#,
        )
        .bind(entry.id)
        .bind(entry.guild_id.as_str())
        .bind(entry.owner_id.as_str())
        .bind(entry.target_id.as_str())
        .bind(&entry.target_name)
        .bind(entry.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn remove_from_whitelist(
        &self,
        guild_id: &str,
        owner_id: &str,
        target_id: &str,
    ) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM voice_channel_whitelists WHERE guild_id = $1 AND owner_id = $2 AND target_id = $3")
            .bind(guild_id)
            .bind(owner_id)
            .bind(target_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }
}
