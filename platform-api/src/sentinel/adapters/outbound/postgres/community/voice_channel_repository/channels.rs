use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannel;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::voice_channel_repository::VoiceChannelStore;

#[derive(sqlx::FromRow)]
struct VoiceChannelRow {
    id: Uuid,
    guild_id: String,
    owner_id: String,
    owner_name: String,
    channel_id: String,
    text_channel_id: Option<String>,
    members_channel_id: Option<String>,
    queue_channel_id: Option<String>,
    category_id: Option<String>,
    channel_name: String,
    kind: crate::sentinel::adapters::outbound::postgres::types::PgVoiceChannelKind,
    visibility: String,
    queue_enabled: bool,
    locked: bool,
    stage_enabled: bool,
    member_limit: Option<i32>,
    status: Option<String>,
    channel_status: String,
    closed_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<VoiceChannelRow> for VoiceChannel {
    fn from(row: VoiceChannelRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id.into(),
            owner_id: row.owner_id,
            owner_name: row.owner_name,
            channel_id: row.channel_id.into(),
            text_channel_id: row.text_channel_id,
            members_channel_id: row.members_channel_id,
            queue_channel_id: row.queue_channel_id,
            category_id: row.category_id,
            channel_name: row.channel_name,
            kind: row.kind.into(),
            visibility: row.visibility,
            queue_enabled: row.queue_enabled,
            locked: row.locked,
            stage_enabled: row.stage_enabled,
            member_limit: row.member_limit,
            status: row.status,
            channel_status: row.channel_status,
            closed_at: row.closed_at,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl VoiceChannelStore for super::PgVoiceChannelRepository {
    async fn find_all(&self) -> Result<Vec<VoiceChannel>, DomainError> {
        let rows = sqlx::query_as::<_, VoiceChannelRow>(
            "SELECT * FROM voice_channels WHERE channel_status = 'open' ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(VoiceChannel::from).collect())
    }

    async fn find_all_by_guild(&self, guild_id: &str) -> Result<Vec<VoiceChannel>, DomainError> {
        let rows = sqlx::query_as::<_, VoiceChannelRow>(
            "SELECT * FROM voice_channels WHERE guild_id = $1 AND channel_status = 'open' ORDER BY created_at DESC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(VoiceChannel::from).collect())
    }

    async fn find_closed_by_guild(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<VoiceChannel>, DomainError> {
        let rows = sqlx::query_as::<_, VoiceChannelRow>(
            "SELECT * FROM voice_channels \
             WHERE guild_id = $1 AND channel_status = 'closed' \
             ORDER BY closed_at DESC NULLS LAST, created_at DESC \
             LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(VoiceChannel::from).collect())
    }

    async fn find_by_channel_id(
        &self,
        channel_id: &str,
    ) -> Result<Option<VoiceChannel>, DomainError> {
        // BUG FIX : on ne filtre plus sur channel_status='open' pour permettre
        // l'acces aux details des salons fermes (historique). Si un appelant
        // a besoin uniquement des salons actifs, qu'il filtre cote applicatif.
        let row = sqlx::query_as::<_, VoiceChannelRow>(
            "SELECT * FROM voice_channels WHERE channel_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(channel_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.map(VoiceChannel::from))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<VoiceChannel>, DomainError> {
        let row =
            sqlx::query_as::<_, VoiceChannelRow>("SELECT * FROM voice_channels WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;

        Ok(row.map(VoiceChannel::from))
    }

    async fn save(&self, channel: &VoiceChannel) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO voice_channels (id, guild_id, owner_id, owner_name, channel_id, text_channel_id, members_channel_id, queue_channel_id, category_id, channel_name, kind, visibility, queue_enabled, locked, stage_enabled, member_limit, status, channel_status, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::voice_channel_kind, $12, $13, $14, $15, $16, $17, $18, $19)
            "#,
        )
        .bind(channel.id)
        .bind(channel.guild_id.as_str())
        .bind(channel.owner_id.as_str())
        .bind(&channel.owner_name)
        .bind(channel.channel_id.as_str())
        .bind(channel.text_channel_id.as_deref())
        .bind(channel.members_channel_id.as_deref())
        .bind(channel.queue_channel_id.as_deref())
        .bind(channel.category_id.as_deref())
        .bind(&channel.channel_name)
        .bind(crate::sentinel::adapters::outbound::postgres::types::PgVoiceChannelKind::from(channel.kind))
        .bind(&channel.visibility)
        .bind(channel.queue_enabled)
        .bind(channel.locked)
        .bind(channel.stage_enabled)
        .bind(channel.member_limit)
        .bind(&channel.status)
        .bind(&channel.channel_status)
        .bind(channel.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn close(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE voice_channels SET channel_status = 'closed', closed_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn close_by_channel_id(&self, channel_id: &str) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE voice_channels SET channel_status = 'closed', closed_at = NOW() WHERE channel_id = $1 AND channel_status = 'open'",
        )
        .bind(channel_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        // Soft-delete : close au lieu de delete
        self.close(id).await
    }

    async fn hard_delete_closed_by_channel_id(&self, channel_id: &str) -> Result<u64, DomainError> {
        let res = sqlx::query(
            "DELETE FROM voice_channels WHERE channel_id = $1 AND channel_status = 'closed'",
        )
        .bind(channel_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(res.rows_affected())
    }

    async fn hard_delete_closed_by_guild(&self, guild_id: &str) -> Result<u64, DomainError> {
        let res = sqlx::query(
            "DELETE FROM voice_channels WHERE guild_id = $1 AND channel_status = 'closed'",
        )
        .bind(guild_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(res.rows_affected())
    }

    async fn update_visibility(&self, id: Uuid, visibility: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET visibility = $1 WHERE id = $2")
            .bind(visibility)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn update_locked(&self, id: Uuid, locked: bool) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET locked = $1 WHERE id = $2")
            .bind(locked)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn update_queue_enabled(&self, id: Uuid, queue_enabled: bool) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET queue_enabled = $1 WHERE id = $2")
            .bind(queue_enabled)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn update_name(&self, id: Uuid, name: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET channel_name = $1 WHERE id = $2")
            .bind(name)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn update_status(&self, id: Uuid, status: Option<&str>) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET status = $1 WHERE id = $2")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn update_member_limit(&self, id: Uuid, limit: Option<i32>) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET member_limit = $1 WHERE id = $2")
            .bind(limit)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn update_owner(
        &self,
        id: Uuid,
        owner_id: &str,
        owner_name: &str,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET owner_id = $1, owner_name = $2 WHERE id = $3")
            .bind(owner_id)
            .bind(owner_name)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn update_queue_channel(
        &self,
        id: Uuid,
        queue_channel_id: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET queue_channel_id = $1 WHERE id = $2")
            .bind(queue_channel_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn update_stage(&self, id: Uuid, stage_enabled: bool) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channels SET stage_enabled = $1 WHERE id = $2")
            .bind(stage_enabled)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }
}
