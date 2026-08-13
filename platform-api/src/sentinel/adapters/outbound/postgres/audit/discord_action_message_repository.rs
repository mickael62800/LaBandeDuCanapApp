//! Adapter Postgres du `DiscordActionMessageRepository` (migration 175).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::sentinel::adapters::outbound::postgres::pg_err;
use platform_core::sentinel::domain::entities::audit::discord_action_message::DiscordActionMessage;
use platform_core::sentinel::domain::entities::audit::discord_action_message::NewDiscordActionMessage;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::audit::discord_action_message_repository::DiscordActionMessageRepository;

pub struct PgDiscordActionMessageRepository {
    pool: PgPool,
}

impl PgDiscordActionMessageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DiscordActionMessageRepository for PgDiscordActionMessageRepository {
    async fn register(&self, msg: NewDiscordActionMessage) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO discord_action_messages
                   (action_id, kind, guild_id, channel_id, message_id, posted_at)
               VALUES ($1, $2, $3, $4, $5, NOW())
               ON CONFLICT (action_id, kind) DO UPDATE
                   SET guild_id = EXCLUDED.guild_id,
                       channel_id = EXCLUDED.channel_id,
                       message_id = EXCLUDED.message_id,
                       posted_at = NOW(),
                       last_edited_at = NULL"#,
        )
        .bind(msg.action_id)
        .bind(&msg.kind)
        .bind(msg.guild_id.as_str())
        .bind(msg.channel_id.as_str())
        .bind(msg.message_id.as_str())
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_for_action(
        &self,
        action_id: Uuid,
    ) -> Result<Vec<DiscordActionMessage>, DomainError> {
        let rows: Vec<(
            Uuid,
            String,
            String,
            String,
            String,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
        )> = sqlx::query_as(
            r#"SELECT action_id, kind, guild_id, channel_id, message_id, posted_at, last_edited_at
                   FROM discord_action_messages
                   WHERE action_id = $1
                   ORDER BY posted_at"#,
        )
        .bind(action_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(row_to_entity).collect())
    }
}

fn row_to_entity(
    row: (
        Uuid,
        String,
        String,
        String,
        String,
        DateTime<Utc>,
        Option<DateTime<Utc>>,
    ),
) -> DiscordActionMessage {
    DiscordActionMessage {
        action_id: row.0,
        kind: row.1,
        guild_id: row.2.into(),
        channel_id: row.3.into(),
        message_id: row.4.into(),
        posted_at: row.5,
        last_edited_at: row.6,
    }
}
