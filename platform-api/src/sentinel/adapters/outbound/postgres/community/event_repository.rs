use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::sentinel::adapters::outbound::postgres::pg_err;
use platform_core::sentinel::domain::entities::community::event::{
    CommunityEvent, EventAnswer, EventParticipant, EventStatus, UpsertEventCommand,
};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::event_repository::{
    EventRepository, EventWindow,
};

const COLS: &str = "id, guild_id, title, description, game, color, starts_at, ends_at, \
                    all_day, is_public, status, created_by, created_at, updated_at";

pub struct PgEventRepository {
    pool: PgPool,
}

impl PgEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    guild_id: String,
    title: String,
    description: Option<String>,
    game: Option<String>,
    color: Option<String>,
    starts_at: chrono::DateTime<chrono::Utc>,
    ends_at: chrono::DateTime<chrono::Utc>,
    all_day: bool,
    is_public: bool,
    status: String,
    created_by: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<EventRow> for CommunityEvent {
    fn from(r: EventRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            title: r.title,
            description: r.description,
            game: r.game,
            color: r.color,
            starts_at: r.starts_at,
            ends_at: r.ends_at,
            all_day: r.all_day,
            is_public: r.is_public,
            status: EventStatus::parse(&r.status),
            created_by: r.created_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ParticipantRow {
    event_id: Uuid,
    user_id: String,
    username: String,
    answer: String,
    registered_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
impl EventRepository for PgEventRepository {
    async fn list_in_window(
        &self,
        guild_id: &str,
        window: EventWindow,
        public_only: bool,
    ) -> Result<Vec<CommunityEvent>, DomainError> {
        // CHEVAUCHEMENT, pas date de debut : une campagne de trois semaines
        // doit ressortir dans chacune des semaines qu'elle couvre.
        let sql = format!(
            "SELECT {COLS} FROM community_events \
             WHERE guild_id = $1 AND starts_at < $3 AND ends_at >= $2 \
               AND ($4 = false OR (is_public = true AND status = 'published')) \
             ORDER BY starts_at ASC"
        );
        let rows: Vec<EventRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(window.from)
            .bind(window.to)
            .bind(public_only)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<CommunityEvent>, DomainError> {
        let row: Option<EventRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM community_events WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn create(&self, cmd: &UpsertEventCommand) -> Result<CommunityEvent, DomainError> {
        let row: EventRow = sqlx::query_as(&format!(
            "INSERT INTO community_events \
                 (guild_id, title, description, game, color, starts_at, ends_at, \
                  all_day, is_public, status, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
             RETURNING {COLS}"
        ))
        .bind(&cmd.guild_id)
        .bind(&cmd.title)
        .bind(&cmd.description)
        .bind(&cmd.game)
        .bind(&cmd.color)
        .bind(cmd.starts_at)
        .bind(cmd.ends_at)
        .bind(cmd.all_day)
        .bind(cmd.is_public)
        .bind(cmd.status.as_str())
        .bind(&cmd.created_by)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.into())
    }

    async fn update(
        &self,
        id: Uuid,
        cmd: &UpsertEventCommand,
    ) -> Result<Option<CommunityEvent>, DomainError> {
        // `created_by` n'est pas mis a jour : l'auteur d'origine reste l'auteur.
        let row: Option<EventRow> = sqlx::query_as(&format!(
            "UPDATE community_events SET \
                 title = $2, description = $3, game = $4, color = $5, \
                 starts_at = $6, ends_at = $7, all_day = $8, is_public = $9, \
                 status = $10, updated_at = now() \
             WHERE id = $1 \
             RETURNING {COLS}"
        ))
        .bind(id)
        .bind(&cmd.title)
        .bind(&cmd.description)
        .bind(&cmd.game)
        .bind(&cmd.color)
        .bind(cmd.starts_at)
        .bind(cmd.ends_at)
        .bind(cmd.all_day)
        .bind(cmd.is_public)
        .bind(cmd.status.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn delete(&self, id: Uuid) -> Result<bool, DomainError> {
        let res = sqlx::query("DELETE FROM community_events WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn list_participants(
        &self,
        event_id: Uuid,
    ) -> Result<Vec<EventParticipant>, DomainError> {
        let rows: Vec<ParticipantRow> = sqlx::query_as(
            "SELECT event_id, user_id, username, answer, registered_at \
             FROM community_event_participants \
             WHERE event_id = $1 ORDER BY registered_at ASC",
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| EventParticipant {
                event_id: r.event_id,
                user_id: r.user_id,
                username: r.username,
                answer: EventAnswer::parse(&r.answer),
                registered_at: r.registered_at,
            })
            .collect())
    }

    async fn set_participation(
        &self,
        event_id: Uuid,
        user_id: &str,
        username: &str,
        answer: EventAnswer,
    ) -> Result<(), DomainError> {
        // Idempotent : changer d'avis met la reponse a jour au lieu d'echouer
        // sur la cle primaire.
        sqlx::query(
            "INSERT INTO community_event_participants (event_id, user_id, username, answer) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (event_id, user_id) \
             DO UPDATE SET answer = EXCLUDED.answer, username = EXCLUDED.username",
        )
        .bind(event_id)
        .bind(user_id)
        .bind(username)
        .bind(answer.as_str())
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn remove_participation(
        &self,
        event_id: Uuid,
        user_id: &str,
    ) -> Result<bool, DomainError> {
        let res = sqlx::query(
            "DELETE FROM community_event_participants WHERE event_id = $1 AND user_id = $2",
        )
        .bind(event_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }
}
