//! Memoire conversationnelle persistante et bornee par membre.

use chrono::Utc;
use sqlx::{PgPool, QueryBuilder, Row};
use uuid::Uuid;

const HISTORY_MESSAGES: i64 = 10;
const STORED_MESSAGES: i64 = 20;
const HISTORY_MAX_CHARS: usize = 4_000;

#[derive(Clone)]
pub struct ConversationMemory {
    pool: PgPool,
}

impl ConversationMemory {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
        // Les deux lignes d'un echange (message du membre + reponse d'Atrium)
        // sont inserees en une seule requete. Le message du membre est omis
        // s'il est vide (accueil sans question).
        let mut rows: Vec<(&str, &str)> = Vec::with_capacity(2);
        if !member_message.trim().is_empty() {
            rows.push(("member", member_message));
        }
        rows.push(("atrium", reply));

        let mut tx = self.pool.begin().await?;
        let mut builder = QueryBuilder::new(
            "INSERT INTO atrium_conversation_messages (guild_id, member_id, role, content) ",
        );
        builder.push_values(rows, |mut row, (role, content)| {
            row.push_bind(guild_id)
                .push_bind(member_id)
                .push_bind(role)
                .push_bind(content);
        });
        builder.build().execute(&mut *tx).await?;
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

    pub async fn get_recent_activity(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<String, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT role, content FROM atrium_conversation_messages WHERE guild_id = $1 ORDER BY id DESC LIMIT $2"
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut activity_log = String::new();
        for row in rows.iter().rev() {
            let role: String = row.try_get("role").unwrap_or_default();
            let content: String = row.try_get("content").unwrap_or_default();
            activity_log.push_str(&format!("{}: {}\n", role, content));
        }

        Ok(activity_log)
    }

    pub async fn save_summary(&self, guild_id: &str, content: &str) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let start_date = now - chrono::Duration::days(7);

        sqlx::query(
            "INSERT INTO atrium_server_summaries (id, guild_id, start_date, end_date, content, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(id)
        .bind(guild_id)
        .bind(start_date)
        .bind(now)
        .bind(content)
        .bind(now)
        .execute(&self.pool)
        .await?;

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
