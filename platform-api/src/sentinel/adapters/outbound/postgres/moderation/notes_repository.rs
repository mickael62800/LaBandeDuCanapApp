use crate::sentinel::adapters::outbound::postgres::pg_ctx;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::moderation::user_note::UserNote;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::moderation::notes_repository::NotesRepository;

pub struct PgNotesRepository {
    pool: PgPool,
}

impl PgNotesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct NoteRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    author_id: String,
    author_name: String,
    content: String,
    category: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<NoteRow> for UserNote {
    fn from(r: NoteRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            user_id: r.user_id.into(),
            author_id: r.author_id,
            author_name: r.author_name,
            content: r.content,
            category: r.category,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl NotesRepository for PgNotesRepository {
    async fn save(&self, note: &UserNote) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO user_notes (id, guild_id, user_id, author_id, author_name, content, category, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(note.id)
        .bind(note.guild_id.as_str())
        .bind(note.user_id.as_str())
        .bind(note.author_id.as_str())
        .bind(&note.author_name)
        .bind(&note.content)
        .bind(&note.category)
        .bind(note.created_at)
        .bind(note.updated_at)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("save_note"))?;
        Ok(())
    }

    async fn find_by_user(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<UserNote>, DomainError> {
        let rows = sqlx::query_as::<_, NoteRow>(
            "SELECT id, guild_id, user_id, author_id, author_name, content, category, created_at, updated_at
             FROM user_notes WHERE guild_id = $1 AND user_id = $2 ORDER BY created_at DESC"
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("find_notes"))?;

        Ok(rows.into_iter().map(UserNote::from).collect())
    }

    async fn delete(&self, note_id: &str) -> Result<(), DomainError> {
        let uuid = Uuid::parse_str(note_id)
            .map_err(|_| DomainError::NotFound(format!("Note {note_id}")))?;

        sqlx::query("DELETE FROM user_notes WHERE id = $1")
            .bind(uuid)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("delete_note"))?;
        Ok(())
    }

    async fn find_guild_id(&self, note_id: &str) -> Result<Option<String>, DomainError> {
        let uuid = match Uuid::parse_str(note_id) {
            Ok(u) => u,
            Err(_) => return Ok(None),
        };
        let row: Option<(String,)> =
            sqlx::query_as("SELECT guild_id FROM user_notes WHERE id = $1")
                .bind(uuid)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_ctx("find_note_guild_id"))?;
        Ok(row.map(|(g,)| g))
    }
}
