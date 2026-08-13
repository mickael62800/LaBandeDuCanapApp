use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelCoAdmin;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::voice_channel_repository::VoiceCoAdminStore;

#[derive(sqlx::FromRow)]
struct CoAdminRow {
    id: Uuid,
    voice_channel_id: Uuid,
    user_id: String,
    user_name: String,
    granted_at: chrono::DateTime<chrono::Utc>,
}

impl From<CoAdminRow> for VoiceChannelCoAdmin {
    fn from(row: CoAdminRow) -> Self {
        Self {
            id: row.id,
            voice_channel_id: row.voice_channel_id,
            user_id: row.user_id.into(),
            user_name: row.user_name,
            granted_at: row.granted_at,
        }
    }
}

#[async_trait]
impl VoiceCoAdminStore for super::PgVoiceChannelRepository {
    async fn find_co_admins(
        &self,
        voice_channel_id: Uuid,
    ) -> Result<Vec<VoiceChannelCoAdmin>, DomainError> {
        let rows = sqlx::query_as::<_, CoAdminRow>(
            "SELECT * FROM voice_channel_co_admins WHERE voice_channel_id = $1 ORDER BY granted_at ASC",
        )
        .bind(voice_channel_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(VoiceChannelCoAdmin::from).collect())
    }

    async fn add_co_admin(&self, co_admin: &VoiceChannelCoAdmin) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO voice_channel_co_admins (id, voice_channel_id, user_id, user_name, granted_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (voice_channel_id, user_id) DO NOTHING
            "#,
        )
        .bind(co_admin.id)
        .bind(co_admin.voice_channel_id)
        .bind(co_admin.user_id.as_str())
        .bind(&co_admin.user_name)
        .bind(co_admin.granted_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn remove_co_admin(
        &self,
        voice_channel_id: Uuid,
        user_id: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "DELETE FROM voice_channel_co_admins WHERE voice_channel_id = $1 AND user_id = $2",
        )
        .bind(voice_channel_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }
}
