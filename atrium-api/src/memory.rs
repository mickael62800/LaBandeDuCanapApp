//! Memoire conversationnelle persistante et bornee par membre.

use sqlx::{PgPool, Row};

use crate::AppConfig;

const HISTORY_MESSAGES: i64 = 10;
const STORED_MESSAGES: i64 = 20;
const HISTORY_MAX_CHARS: usize = 4_000;

#[derive(Clone)]
pub struct ConversationMemory {
    pool: PgPool,
}

impl ConversationMemory {
    pub fn new(config: &AppConfig) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: PgPool::connect_lazy(&config.rag_database_url)?,
        })
    }

    pub async fn history(&self, guild_id: &str, member_id: &str) -> Result<String, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT role, content FROM (\
                SELECT id, role, content FROM atrium_conversation_messages \
                WHERE guild_id = $1 AND member_id = $2 ORDER BY id DESC LIMIT $3\
             ) recent ORDER BY id ASC",
        )
        .bind(guild_id)
        .bind(member_id)
        .bind(HISTORY_MESSAGES)
        .fetch_all(&self.pool)
        .await?;

        let mut history = String::new();
        for row in rows {
            let role: String = row.try_get("role")?;
            let content: String = row.try_get("content")?;
            history.push_str(if role == "atrium" {
                "Atrium: "
            } else {
                "Membre: "
            });
            history.push_str(&content);
            history.push('\n');
        }
        let char_count = history.chars().count();
        Ok(history
            .chars()
            .skip(char_count.saturating_sub(HISTORY_MAX_CHARS))
            .collect())
    }

    pub async fn remember_exchange(
        &self,
        guild_id: &str,
        member_id: &str,
        member_message: &str,
        reply: &str,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        if !member_message.trim().is_empty() {
            sqlx::query(
                "INSERT INTO atrium_conversation_messages (guild_id, member_id, role, content) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(guild_id)
            .bind(member_id)
            .bind("member")
            .bind(member_message)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO atrium_conversation_messages (guild_id, member_id, role, content) \
             VALUES ($1, $2, 'atrium', $3)",
        )
        .bind(guild_id)
        .bind(member_id)
        .bind(reply)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE FROM atrium_conversation_messages WHERE guild_id = $1 AND member_id = $2 \
             AND id NOT IN (SELECT id FROM atrium_conversation_messages \
             WHERE guild_id = $1 AND member_id = $2 ORDER BY id DESC LIMIT $3)",
        )
        .bind(guild_id)
        .bind(member_id)
        .bind(STORED_MESSAGES)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_latest_summary(&self, guild_id: &str) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT content FROM atrium_server_summaries WHERE guild_id = $1 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(guild_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get("content")))
    }
}
