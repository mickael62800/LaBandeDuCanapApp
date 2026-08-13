use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::sentinel::adapters::outbound::postgres::pg_err;
use platform_core::sentinel::domain::entities::community::lfg::{
    LfgInterest, LfgPost, UpsertLfgCommand,
};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::lfg_repository::LfgRepository;

const COLS: &str = "id, guild_id, author_id, author_name, game, game_server_id, slots, \
                    when_text, description, is_open, expires_at, created_at";

pub struct PgLfgRepository {
    pool: PgPool,
}

impl PgLfgRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Charge les interesses de plusieurs annonces en une requete.
    ///
    /// Une requete par annonce (N+1) serait invisible avec trois annonces et
    /// catastrophique avec cinquante — et cette section est celle qu'on
    /// consulte le plus souvent.
    async fn load_interests(&self, ids: &[Uuid]) -> Result<Vec<(Uuid, LfgInterest)>, DomainError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<InterestRow> = sqlx::query_as(
            "SELECT lfg_id, user_id, username, joined_at \
             FROM community_lfg_interest \
             WHERE lfg_id = ANY($1) \
             ORDER BY joined_at ASC",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.lfg_id,
                    LfgInterest {
                        user_id: r.user_id,
                        username: r.username,
                        joined_at: r.joined_at,
                    },
                )
            })
            .collect())
    }

    /// Assemble les lignes et leurs interesses.
    async fn hydrate(&self, rows: Vec<LfgRow>) -> Result<Vec<LfgPost>, DomainError> {
        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let interests = self.load_interests(&ids).await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let interested = interests
                    .iter()
                    .filter(|(id, _)| *id == r.id)
                    .map(|(_, i)| i.clone())
                    .collect();
                r.into_post(interested)
            })
            .collect())
    }
}

#[derive(sqlx::FromRow)]
struct LfgRow {
    id: Uuid,
    guild_id: String,
    author_id: String,
    author_name: String,
    game: String,
    game_server_id: Option<Uuid>,
    slots: i32,
    when_text: String,
    description: Option<String>,
    is_open: bool,
    expires_at: chrono::DateTime<chrono::Utc>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl LfgRow {
    fn into_post(self, interested: Vec<LfgInterest>) -> LfgPost {
        LfgPost {
            id: self.id,
            guild_id: self.guild_id,
            author_id: self.author_id,
            author_name: self.author_name,
            game: self.game,
            game_server_id: self.game_server_id,
            slots: self.slots,
            when_text: self.when_text,
            description: self.description,
            is_open: self.is_open,
            expires_at: self.expires_at,
            created_at: self.created_at,
            interested,
        }
    }
}

#[derive(sqlx::FromRow)]
struct InterestRow {
    lfg_id: Uuid,
    user_id: String,
    username: String,
    joined_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
impl LfgRepository for PgLfgRepository {
    async fn list(
        &self,
        guild_id: &str,
        live_only: bool,
        limit: i64,
    ) -> Result<Vec<LfgPost>, DomainError> {
        // L'expiration est filtree en SQL et non apres coup : sinon la limite
        // serait consommee par des annonces mortes et la page en afficherait
        // moins que demande.
        let sql = format!(
            "SELECT {COLS} FROM community_lfg \
             WHERE guild_id = $1 \
               AND ($2 = false OR (is_open = true AND expires_at > now())) \
             ORDER BY created_at DESC \
             LIMIT $3"
        );
        let rows: Vec<LfgRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(live_only)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        self.hydrate(rows).await
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<LfgPost>, DomainError> {
        let row: Option<LfgRow> =
            sqlx::query_as(&format!("SELECT {COLS} FROM community_lfg WHERE id = $1"))
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;

        match row {
            Some(r) => Ok(self.hydrate(vec![r]).await?.into_iter().next()),
            None => Ok(None),
        }
    }

    async fn create(&self, cmd: &UpsertLfgCommand) -> Result<LfgPost, DomainError> {
        let row: LfgRow = sqlx::query_as(&format!(
            "INSERT INTO community_lfg \
                 (guild_id, author_id, author_name, game, game_server_id, slots, \
                  when_text, description, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             RETURNING {COLS}"
        ))
        .bind(&cmd.guild_id)
        .bind(&cmd.author_id)
        .bind(&cmd.author_name)
        .bind(&cmd.game)
        .bind(cmd.game_server_id)
        .bind(cmd.slots)
        .bind(&cmd.when_text)
        .bind(&cmd.description)
        .bind(cmd.resolved_expiry(chrono::Utc::now()))
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;

        // Une annonce fraiche n'a evidemment aucun interesse : pas de requete.
        Ok(row.into_post(vec![]))
    }

    async fn set_open(&self, id: Uuid, open: bool) -> Result<bool, DomainError> {
        let res = sqlx::query("UPDATE community_lfg SET is_open = $2 WHERE id = $1")
            .bind(id)
            .bind(open)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn delete(&self, id: Uuid) -> Result<bool, DomainError> {
        let res = sqlx::query("DELETE FROM community_lfg WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn add_interest(
        &self,
        id: Uuid,
        user_id: &str,
        username: &str,
    ) -> Result<(), DomainError> {
        // Idempotent : cliquer deux fois rafraichit le pseudo au lieu
        // d'echouer sur la cle primaire.
        sqlx::query(
            "INSERT INTO community_lfg_interest (lfg_id, user_id, username) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (lfg_id, user_id) DO UPDATE SET username = EXCLUDED.username",
        )
        .bind(id)
        .bind(user_id)
        .bind(username)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn remove_interest(&self, id: Uuid, user_id: &str) -> Result<bool, DomainError> {
        let res =
            sqlx::query("DELETE FROM community_lfg_interest WHERE lfg_id = $1 AND user_id = $2")
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn purge_expired(&self, older_than_hours: i64) -> Result<u64, DomainError> {
        // Delai de grace apres l'expiration : l'annonce disparait de
        // l'affichage tout de suite, mais reste en base le temps qu'un membre
        // retrouve avec qui il avait rendez-vous.
        let res = sqlx::query(
            "DELETE FROM community_lfg \
             WHERE expires_at < now() - make_interval(hours => $1::int)",
        )
        .bind(older_than_hours as i32)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(res.rows_affected())
    }
}
