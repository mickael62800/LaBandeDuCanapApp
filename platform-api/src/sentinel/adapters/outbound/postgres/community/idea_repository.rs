use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, QueryBuilder};
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::idea::{Idea, IdeaMessage};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::idea_repository::{
    IdeaFilters, IdeaRepository,
};

pub struct PgIdeaRepository {
    pool: PgPool,
}

impl PgIdeaRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct IdeaRow {
    id: Uuid,
    guild_id: String,
    title: String,
    description: String,
    status: String,
    category: String,
    author_id: String,
    author_name: String,
    channel_id: Option<String>,
    decided_by: Option<String>,
    decided_by_name: Option<String>,
    decision_reason: Option<String>,
    decided_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<IdeaRow> for Idea {
    fn from(r: IdeaRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            title: r.title,
            description: r.description,
            status: r.status,
            category: r.category,
            author_id: r.author_id,
            author_name: r.author_name,
            channel_id: r.channel_id,
            decided_by: r.decided_by,
            decided_by_name: r.decided_by_name,
            decision_reason: r.decision_reason,
            decided_at: r.decided_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(FromRow)]
struct IdeaMessageRow {
    id: Uuid,
    idea_id: Uuid,
    author_name: String,
    author_role: String,
    content: String,
    created_at: DateTime<Utc>,
}

impl From<IdeaMessageRow> for IdeaMessage {
    fn from(r: IdeaMessageRow) -> Self {
        Self {
            id: r.id,
            idea_id: r.idea_id,
            author_name: r.author_name,
            author_role: r.author_role,
            content: r.content,
            created_at: r.created_at,
        }
    }
}

const SELECT_IDEA: &str = r#"
    SELECT id, guild_id, title, description, status, category,
        author_id, author_name, channel_id,
        decided_by, decided_by_name, decision_reason, decided_at,
        created_at, updated_at
    FROM ideas
"#;

#[async_trait]
impl IdeaRepository for PgIdeaRepository {
    async fn find_all(
        &self,
        filters: IdeaFilters<'_>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Idea>, DomainError> {
        // QueryBuilder : les valeurs restent des binds ($n), jamais concatenees.
        let mut qb = QueryBuilder::new(SELECT_IDEA);
        qb.push(" WHERE 1 = 1");
        if let Some(g) = filters.guild_id {
            qb.push(" AND guild_id = ").push_bind(g);
        }
        if let Some(s) = filters.status {
            qb.push(" AND status = ").push_bind(s);
        }
        if let Some(c) = filters.category {
            qb.push(" AND category = ").push_bind(c);
        }
        if let Some(a) = filters.author_id {
            qb.push(" AND author_id = ").push_bind(a);
        }
        if let Some(search) = filters.search {
            // `%` et `_` du terme sont echappes pour rester litteraux.
            let pattern = format!(
                "%{}%",
                search
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            );
            qb.push(" AND (title ILIKE ")
                .push_bind(pattern.clone())
                .push(" OR description ILIKE ")
                .push_bind(pattern)
                .push(")");
        }
        qb.push(" ORDER BY created_at DESC LIMIT ")
            .push_bind(limit)
            .push(" OFFSET ")
            .push_bind(offset);

        let rows: Vec<IdeaRow> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(Idea::from).collect())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Idea>, DomainError> {
        let row: Option<IdeaRow> = sqlx::query_as(&format!("{SELECT_IDEA} WHERE id = $1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Idea::from))
    }

    async fn find_by_channel(&self, channel_id: &str) -> Result<Option<Idea>, DomainError> {
        let row: Option<IdeaRow> = sqlx::query_as(&format!("{SELECT_IDEA} WHERE channel_id = $1"))
            .bind(channel_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Idea::from))
    }

    async fn create(&self, idea: &Idea) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO ideas (
                id, guild_id, title, description, status, category,
                author_id, author_name, channel_id,
                decided_by, decided_by_name, decision_reason, decided_at,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
            )"#,
        )
        .bind(idea.id)
        .bind(&idea.guild_id)
        .bind(&idea.title)
        .bind(&idea.description)
        .bind(&idea.status)
        .bind(&idea.category)
        .bind(&idea.author_id)
        .bind(&idea.author_name)
        .bind(&idea.channel_id)
        .bind(&idea.decided_by)
        .bind(&idea.decided_by_name)
        .bind(&idea.decision_reason)
        .bind(idea.decided_at)
        .bind(idea.created_at)
        .bind(idea.updated_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn update(&self, idea: &Idea) -> Result<(), DomainError> {
        sqlx::query(
            r#"UPDATE ideas SET
                title = $2, description = $3, status = $4, category = $5,
                channel_id = $6, decided_by = $7, decided_by_name = $8,
                decision_reason = $9, decided_at = $10, updated_at = $11
            WHERE id = $1"#,
        )
        .bind(idea.id)
        .bind(&idea.title)
        .bind(&idea.description)
        .bind(&idea.status)
        .bind(&idea.category)
        .bind(&idea.channel_id)
        .bind(&idea.decided_by)
        .bind(&idea.decided_by_name)
        .bind(&idea.decision_reason)
        .bind(idea.decided_at)
        .bind(idea.updated_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        // Les messages partent en cascade (FK ON DELETE CASCADE).
        sqlx::query("DELETE FROM ideas WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn count_open_by_author(
        &self,
        guild_id: &str,
        author_id: &str,
    ) -> Result<i64, DomainError> {
        let (count,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM ideas
               WHERE guild_id = $1 AND author_id = $2
                 AND status IN ('nouvelle', 'en_discussion')"#,
        )
        .bind(guild_id)
        .bind(author_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(count)
    }

    async fn find_messages(&self, idea_id: Uuid) -> Result<Vec<IdeaMessage>, DomainError> {
        let rows: Vec<IdeaMessageRow> = sqlx::query_as(
            r#"SELECT id, idea_id, author_name, author_role, content, created_at
               FROM idea_messages WHERE idea_id = $1 ORDER BY created_at ASC"#,
        )
        .bind(idea_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(IdeaMessage::from).collect())
    }

    async fn save_message(&self, message: &IdeaMessage) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO idea_messages (id, idea_id, author_name, author_role, content, created_at)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(message.id)
        .bind(message.idea_id)
        .bind(&message.author_name)
        .bind(&message.author_role)
        .bind(&message.content)
        .bind(message.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
