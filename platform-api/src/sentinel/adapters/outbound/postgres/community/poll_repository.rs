use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::sentinel::adapters::outbound::postgres::pg_err;
use platform_core::sentinel::domain::entities::community::poll::{
    Poll, PollOption, UpsertPollCommand,
};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::poll_repository::PollRepository;

const COLS: &str = "id, guild_id, question, description, closes_at, is_closed, \
                    is_public, created_by, created_at";

pub struct PgPollRepository {
    pool: PgPool,
}

impl PgPollRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Options de plusieurs sondages, decompte des voix inclus, en une
    /// requete. Le LEFT JOIN garde les options sans aucune voix — sans lui,
    /// un choix a zero disparaitrait du sondage.
    async fn load_options(&self, ids: &[Uuid]) -> Result<Vec<(Uuid, PollOption)>, DomainError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<OptionRow> = sqlx::query_as(
            "SELECT o.poll_id, o.id, o.label, o.color, o.position, \
                    COUNT(v.user_id) AS votes \
             FROM community_poll_options o \
             LEFT JOIN community_poll_votes v ON v.option_id = o.id \
             WHERE o.poll_id = ANY($1) \
             GROUP BY o.poll_id, o.id, o.label, o.color, o.position \
             ORDER BY o.position ASC",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.poll_id,
                    PollOption {
                        id: r.id,
                        label: r.label,
                        color: r.color,
                        position: r.position,
                        votes: r.votes,
                    },
                )
            })
            .collect())
    }

    async fn hydrate(&self, rows: Vec<PollRow>) -> Result<Vec<Poll>, DomainError> {
        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let options = self.load_options(&ids).await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let mine = options
                    .iter()
                    .filter(|(pid, _)| *pid == r.id)
                    .map(|(_, o)| o.clone())
                    .collect();
                r.into_poll(mine)
            })
            .collect())
    }
}

#[derive(sqlx::FromRow)]
struct PollRow {
    id: Uuid,
    guild_id: String,
    question: String,
    description: Option<String>,
    closes_at: chrono::DateTime<chrono::Utc>,
    is_closed: bool,
    is_public: bool,
    created_by: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl PollRow {
    fn into_poll(self, options: Vec<PollOption>) -> Poll {
        Poll {
            id: self.id,
            guild_id: self.guild_id,
            question: self.question,
            description: self.description,
            closes_at: self.closes_at,
            is_closed: self.is_closed,
            is_public: self.is_public,
            created_by: self.created_by,
            created_at: self.created_at,
            options,
        }
    }
}

#[derive(sqlx::FromRow)]
struct OptionRow {
    poll_id: Uuid,
    id: Uuid,
    label: String,
    color: Option<String>,
    position: i32,
    votes: i64,
}

#[async_trait]
impl PollRepository for PgPollRepository {
    async fn list(
        &self,
        guild_id: &str,
        open_only: bool,
        limit: i64,
    ) -> Result<Vec<Poll>, DomainError> {
        let sql = format!(
            "SELECT {COLS} FROM community_polls \
             WHERE guild_id = $1 \
               AND ($2 = false OR (is_closed = false AND closes_at > now())) \
             ORDER BY closes_at ASC, created_at DESC \
             LIMIT $3"
        );
        let rows: Vec<PollRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(open_only)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        self.hydrate(rows).await
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Poll>, DomainError> {
        let row: Option<PollRow> =
            sqlx::query_as(&format!("SELECT {COLS} FROM community_polls WHERE id = $1"))
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;

        match row {
            Some(r) => Ok(self.hydrate(vec![r]).await?.into_iter().next()),
            None => Ok(None),
        }
    }

    async fn create(&self, cmd: &UpsertPollCommand) -> Result<Poll, DomainError> {
        // Transaction : un sondage enregistre sans ses options serait un
        // sondage sur lequel on ne peut pas voter. Les deux ecritures
        // reussissent ou echouent ensemble.
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        let row: PollRow = sqlx::query_as(&format!(
            "INSERT INTO community_polls \
                 (guild_id, question, description, closes_at, is_public, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING {COLS}"
        ))
        .bind(&cmd.guild_id)
        .bind(&cmd.question)
        .bind(&cmd.description)
        .bind(cmd.closes_at)
        .bind(cmd.is_public)
        .bind(&cmd.created_by)
        .fetch_one(&mut *tx)
        .await
        .map_err(pg_err)?;

        let mut options = Vec::with_capacity(cmd.options.len());
        for (i, (label, color)) in cmd.options.iter().enumerate() {
            let opt: OptionInsertRow = sqlx::query_as(
                "INSERT INTO community_poll_options (poll_id, label, color, position) \
                 VALUES ($1, $2, $3, $4) \
                 RETURNING id, label, color, position",
            )
            .bind(row.id)
            .bind(label)
            .bind(color)
            .bind(i as i32)
            .fetch_one(&mut *tx)
            .await
            .map_err(pg_err)?;

            options.push(PollOption {
                id: opt.id,
                label: opt.label,
                color: opt.color,
                position: opt.position,
                votes: 0,
            });
        }

        tx.commit().await.map_err(pg_err)?;
        Ok(row.into_poll(options))
    }

    async fn set_closed(&self, id: Uuid, closed: bool) -> Result<bool, DomainError> {
        let res = sqlx::query("UPDATE community_polls SET is_closed = $2 WHERE id = $1")
            .bind(id)
            .bind(closed)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn delete(&self, id: Uuid) -> Result<bool, DomainError> {
        let res = sqlx::query("DELETE FROM community_polls WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn cast_vote(
        &self,
        poll_id: Uuid,
        option_id: Uuid,
        user_id: &str,
    ) -> Result<bool, DomainError> {
        // Le SELECT dans le INSERT verifie que l'option appartient bien au
        // sondage. Le faire ici plutot qu'en deux requetes evite qu'une
        // suppression d'option entre les deux ne laisse passer un vote
        // orphelin — et l'insertion n'affecte alors aucune ligne.
        let res = sqlx::query(
            "INSERT INTO community_poll_votes (poll_id, option_id, user_id) \
             SELECT $1, $2, $3 \
             WHERE EXISTS ( \
                 SELECT 1 FROM community_poll_options \
                 WHERE id = $2 AND poll_id = $1 \
             ) \
             ON CONFLICT (poll_id, user_id) \
             DO UPDATE SET option_id = EXCLUDED.option_id, voted_at = now()",
        )
        .bind(poll_id)
        .bind(option_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(res.rows_affected() > 0)
    }

    async fn vote_of(&self, poll_id: Uuid, user_id: &str) -> Result<Option<Uuid>, DomainError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT option_id FROM community_poll_votes \
             WHERE poll_id = $1 AND user_id = $2",
        )
        .bind(poll_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(|r| r.0))
    }
}

#[derive(sqlx::FromRow)]
struct OptionInsertRow {
    id: Uuid,
    label: String,
    color: Option<String>,
    position: i32,
}
