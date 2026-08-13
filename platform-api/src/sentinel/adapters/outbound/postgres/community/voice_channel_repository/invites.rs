use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelInviteLink;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::voice_channel_repository::VoiceInviteStore;

#[derive(sqlx::FromRow)]
struct InviteLinkRow {
    id: Uuid,
    voice_channel_id: Uuid,
    guild_id: String,
    channel_id: String,
    created_by: String,
    created_by_name: String,
    code: String,
    max_uses: Option<i32>,
    current_uses: i32,
    expires_at: chrono::DateTime<chrono::Utc>,
    revoked: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<InviteLinkRow> for VoiceChannelInviteLink {
    fn from(row: InviteLinkRow) -> Self {
        Self {
            id: row.id,
            voice_channel_id: row.voice_channel_id,
            guild_id: row.guild_id.into(),
            channel_id: row.channel_id.into(),
            created_by: row.created_by,
            created_by_name: row.created_by_name,
            code: row.code,
            max_uses: row.max_uses,
            current_uses: row.current_uses,
            expires_at: row.expires_at,
            revoked: row.revoked,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl VoiceInviteStore for super::PgVoiceChannelRepository {
    async fn find_invite_links(
        &self,
        voice_channel_id: Uuid,
    ) -> Result<Vec<VoiceChannelInviteLink>, DomainError> {
        let rows = sqlx::query_as::<_, InviteLinkRow>(
            "SELECT * FROM voice_channel_invite_links WHERE voice_channel_id = $1 AND revoked = false AND expires_at > NOW() ORDER BY created_at DESC",
        )
        .bind(voice_channel_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(VoiceChannelInviteLink::from).collect())
    }

    async fn find_invite_by_code(
        &self,
        code: &str,
    ) -> Result<Option<VoiceChannelInviteLink>, DomainError> {
        let row = sqlx::query_as::<_, InviteLinkRow>(
            "SELECT * FROM voice_channel_invite_links WHERE code = $1",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.map(VoiceChannelInviteLink::from))
    }

    async fn save_invite_link(&self, link: &VoiceChannelInviteLink) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO voice_channel_invite_links (id, voice_channel_id, guild_id, channel_id, created_by, created_by_name, code, max_uses, current_uses, expires_at, revoked, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(link.id)
        .bind(link.voice_channel_id)
        .bind(link.guild_id.as_str())
        .bind(link.channel_id.as_str())
        .bind(link.created_by.as_str())
        .bind(&link.created_by_name)
        .bind(&link.code)
        .bind(link.max_uses)
        .bind(link.current_uses)
        .bind(link.expires_at)
        .bind(link.revoked)
        .bind(link.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn increment_invite_uses(&self, id: Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query(
            "UPDATE voice_channel_invite_links SET current_uses = current_uses + 1 WHERE id = $1 AND revoked = false AND expires_at > NOW() AND (max_uses IS NULL OR current_uses < max_uses)",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(result.rows_affected() > 0)
    }

    async fn revoke_invite_link(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE voice_channel_invite_links SET revoked = true WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }
}
