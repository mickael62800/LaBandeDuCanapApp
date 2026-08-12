use crate::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::system::ticket::Ticket;
use sentinel_core::domain::entities::system::ticket::TicketMessage;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::system::ticket_repository::TicketRepository;

pub struct PgTicketRepository {
    pool: PgPool,
}

impl PgTicketRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct TicketRow {
    id: Uuid,
    title: String,
    status: String,
    priority: String,
    author_id: String,
    author_name: String,
    assigned_to: Option<String>,
    server: String,
    guild_id: Option<String>,
    category: String,
    ticket_type: String,
    channel_id: Option<String>,
    voice_channel_id: Option<String>,
    invited_user_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    messages_count: Option<i64>,
}

impl From<TicketRow> for Ticket {
    fn from(row: TicketRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            status: row.status,
            priority: row.priority,
            author_id: row.author_id,
            author_name: row.author_name,
            assigned_to: row.assigned_to,
            server: row.server,
            guild_id: row.guild_id,
            category: row.category,
            ticket_type: row.ticket_type,
            channel_id: row.channel_id,
            voice_channel_id: row.voice_channel_id,
            invited_user_id: row.invited_user_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            messages_count: row.messages_count.unwrap_or(0) as u32,
        }
    }
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: Uuid,
    ticket_id: Uuid,
    author_name: String,
    author_role: String,
    content: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<MessageRow> for TicketMessage {
    fn from(row: MessageRow) -> Self {
        Self {
            id: row.id,
            ticket_id: row.ticket_id,
            author_name: row.author_name,
            author_role: row.author_role,
            content: row.content,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl TicketRepository for PgTicketRepository {
    async fn find_all(
        &self,
        status: Option<&str>,
        priority: Option<&str>,
        search: Option<&str>,
        author_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Ticket>, DomainError> {
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            r#"
            SELECT t.id, t.title, t.status, t.priority, t.author_id, t.author_name,
                   t.assigned_to, t.server, t.guild_id, t.category, t.ticket_type,
                   t.channel_id, t.voice_channel_id, t.invited_user_id,
                   t.created_at, t.updated_at,
                   COUNT(tm.id) AS messages_count
            FROM tickets t
            LEFT JOIN ticket_messages tm ON tm.ticket_id = t.id
            WHERE 1=1
            "#,
        );

        if let Some(s) = status {
            qb.push(" AND t.status = ");
            qb.push_bind(s.to_string());
        }
        if let Some(p) = priority {
            qb.push(" AND t.priority = ");
            qb.push_bind(p.to_string());
        }
        if let Some(s) = search {
            qb.push(" AND LOWER(t.title) LIKE LOWER(");
            qb.push_bind(format!("%{s}%"));
            qb.push(")");
        }
        if let Some(a) = author_id {
            qb.push(" AND t.author_id = ");
            qb.push_bind(a.to_string());
        }

        qb.push(
            r#"
            GROUP BY t.id, t.title, t.status, t.priority, t.author_id, t.author_name,
                     t.assigned_to, t.server, t.guild_id, t.category, t.ticket_type,
                     t.channel_id, t.voice_channel_id, t.invited_user_id,
                     t.created_at, t.updated_at
            ORDER BY t.updated_at DESC
            LIMIT "#,
        );
        qb.push_bind(limit);
        qb.push(" OFFSET ");
        qb.push_bind(offset);

        let rows = qb
            .build_query_as::<TicketRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(rows.into_iter().map(Ticket::from).collect())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Ticket>, DomainError> {
        let row = sqlx::query_as::<_, TicketRow>(
            r#"
            SELECT t.id, t.title, t.status, t.priority, t.author_id, t.author_name,
                   t.assigned_to, t.server, t.guild_id, t.category, t.ticket_type,
                   t.channel_id, t.voice_channel_id, t.invited_user_id,
                   t.created_at, t.updated_at,
                   COUNT(tm.id) AS messages_count
            FROM tickets t
            LEFT JOIN ticket_messages tm ON tm.ticket_id = t.id
            WHERE t.id = $1
            GROUP BY t.id, t.title, t.status, t.priority, t.author_id, t.author_name,
                     t.assigned_to, t.server, t.guild_id, t.category, t.ticket_type,
                     t.channel_id, t.voice_channel_id, t.invited_user_id,
                     t.created_at, t.updated_at
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.map(Ticket::from))
    }

    async fn save(&self, ticket: &Ticket) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO tickets (id, title, status, priority, author_id, author_name, assigned_to, server, guild_id, category, ticket_type, channel_id, voice_channel_id, invited_user_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            "#,
        )
        .bind(ticket.id)
        .bind(&ticket.title)
        .bind(&ticket.status)
        .bind(&ticket.priority)
        .bind(ticket.author_id.as_str())
        .bind(&ticket.author_name)
        .bind(&ticket.assigned_to)
        .bind(&ticket.server)
        .bind(&ticket.guild_id)
        .bind(&ticket.category)
        .bind(&ticket.ticket_type)
        .bind(ticket.channel_id.as_deref())
        .bind(&ticket.voice_channel_id)
        .bind(&ticket.invited_user_id)
        .bind(ticket.created_at)
        .bind(ticket.updated_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn update_status(&self, id: Uuid, status: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE tickets SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn close_if_open(&self, id: Uuid) -> Result<bool, DomainError> {
        let res = sqlx::query(
            "UPDATE tickets SET status = 'closed', updated_at = NOW() \
             WHERE id = $1 AND status <> 'closed'",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(res.rows_affected() == 1)
    }

    async fn update_assignee(&self, id: Uuid, assignee: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE tickets SET assigned_to = $1, updated_at = NOW() WHERE id = $2")
            .bind(assignee)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn find_messages(&self, ticket_id: Uuid) -> Result<Vec<TicketMessage>, DomainError> {
        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT * FROM ticket_messages WHERE ticket_id = $1 ORDER BY created_at ASC",
        )
        .bind(ticket_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(TicketMessage::from).collect())
    }

    async fn save_message(&self, message: &TicketMessage) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ticket_messages (id, ticket_id, author_name, author_role, content, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(message.id)
        .bind(message.ticket_id)
        .bind(&message.author_name)
        .bind(&message.author_role)
        .bind(&message.content)
        .bind(message.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn update_voice_channel(
        &self,
        id: Uuid,
        voice_channel_id: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE tickets SET voice_channel_id = $1, updated_at = NOW() WHERE id = $2")
            .bind(voice_channel_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn update_invited_user(
        &self,
        id: Uuid,
        invited_user_id: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE tickets SET invited_user_id = $1, updated_at = NOW() WHERE id = $2")
            .bind(invited_user_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn update_priority(&self, id: Uuid, priority: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE tickets SET priority = $1, updated_at = NOW() WHERE id = $2")
            .bind(priority)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn update_sla(
        &self,
        id: Uuid,
        first_response_at: Option<&str>,
        resolved_at: Option<&str>,
        satisfaction_rating: Option<i32>,
    ) -> Result<(), DomainError> {
        if let Some(fr) = first_response_at {
            sqlx::query("UPDATE tickets SET first_response_at = $1::timestamptz, updated_at = NOW() WHERE id = $2")
                .bind(fr).bind(id).execute(&self.pool).await
                .map_err(pg_err)?;
        }
        if let Some(ra) = resolved_at {
            sqlx::query("UPDATE tickets SET resolved_at = $1::timestamptz, updated_at = NOW() WHERE id = $2")
                .bind(ra).bind(id).execute(&self.pool).await
                .map_err(pg_err)?;
        }
        if let Some(rating) = satisfaction_rating {
            // Ecriture UNIQUE : on ne pose la note que si aucune n'existe deja
            // -> l'auteur ne peut pas recliquer indefiniment pour manipuler la
            // note (spam de 5/5 ou 1/5).
            sqlx::query(
                "UPDATE tickets SET satisfaction_rating = $1, updated_at = NOW() \
                 WHERE id = $2 AND satisfaction_rating IS NULL",
            )
            .bind(rating)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        }
        Ok(())
    }

    async fn bulk_delete(
        &self,
        author_id: Option<&str>,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<u64, DomainError> {
        // DELETE avec filtres dynamiques via clauses COALESCE-neutres si NULL.
        let res = sqlx::query(
            r#"
            DELETE FROM tickets
            WHERE ($1::text IS NULL OR author_id = $1)
              AND ($2::timestamptz IS NULL OR created_at >= $2)
              AND ($3::timestamptz IS NULL OR created_at <= $3)
            "#,
        )
        .bind(author_id)
        .bind(from)
        .bind(to)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(res.rows_affected())
    }
}
