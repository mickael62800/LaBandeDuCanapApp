use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::sentinel::adapters::outbound::postgres::pg_err;
use platform_core::sentinel::domain::entities::community::news::{NewsPost, UpsertNewsCommand};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::news_repository::NewsRepository;

const COLS: &str = "id, guild_id, title, body, image_url, is_pinned, is_public, \
                    published_at, created_by, created_at";

pub struct PgNewsRepository {
    pool: PgPool,
}

impl PgNewsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct NewsRow {
    id: Uuid,
    guild_id: String,
    title: String,
    body: String,
    image_url: Option<String>,
    is_pinned: bool,
    is_public: bool,
    published_at: chrono::DateTime<chrono::Utc>,
    created_by: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<NewsRow> for NewsPost {
    fn from(r: NewsRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            title: r.title,
            body: r.body,
            image_url: r.image_url,
            is_pinned: r.is_pinned,
            is_public: r.is_public,
            published_at: r.published_at,
            created_by: r.created_by,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl NewsRepository for PgNewsRepository {
    async fn list(
        &self,
        guild_id: &str,
        published_only: bool,
        limit: i64,
    ) -> Result<Vec<NewsPost>, DomainError> {
        // L'epingle l'emporte sur la date : une information importante reste
        // en tete meme quand des nouvelles plus recentes arrivent.
        let sql = format!(
            "SELECT {COLS} FROM community_news \
             WHERE guild_id = $1 \
               AND ($2 = false OR (is_public = true AND published_at <= now())) \
             ORDER BY is_pinned DESC, published_at DESC \
             LIMIT $3"
        );
        let rows: Vec<NewsRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(published_only)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<NewsPost>, DomainError> {
        let row: Option<NewsRow> =
            sqlx::query_as(&format!("SELECT {COLS} FROM community_news WHERE id = $1"))
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn create(&self, cmd: &UpsertNewsCommand) -> Result<NewsPost, DomainError> {
        // `COALESCE($8, now())` plutot qu'un `unwrap_or(Utc::now())` cote
        // Rust : la date de publication par defaut suit l'horloge de la base,
        // comme `created_at`, pour qu'elles ne divergent pas.
        let row: NewsRow = sqlx::query_as(&format!(
            "INSERT INTO community_news \
                 (guild_id, title, body, image_url, is_pinned, is_public, \
                  published_at, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, now()), $8) \
             RETURNING {COLS}"
        ))
        .bind(&cmd.guild_id)
        .bind(&cmd.title)
        .bind(&cmd.body)
        .bind(&cmd.image_url)
        .bind(cmd.is_pinned)
        .bind(cmd.is_public)
        .bind(cmd.published_at)
        .bind(&cmd.created_by)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.into())
    }

    async fn update(
        &self,
        id: Uuid,
        cmd: &UpsertNewsCommand,
    ) -> Result<Option<NewsPost>, DomainError> {
        // `created_by` n'est pas touche : l'auteur d'origine reste l'auteur.
        // `published_at` absent conserve la date existante plutot que de la
        // remettre a maintenant — corriger une faute de frappe ne doit pas
        // faire remonter la nouvelle en tete de liste.
        let row: Option<NewsRow> = sqlx::query_as(&format!(
            "UPDATE community_news SET \
                 title = $2, body = $3, image_url = $4, is_pinned = $5, \
                 is_public = $6, published_at = COALESCE($7, published_at) \
             WHERE id = $1 \
             RETURNING {COLS}"
        ))
        .bind(id)
        .bind(&cmd.title)
        .bind(&cmd.body)
        .bind(&cmd.image_url)
        .bind(cmd.is_pinned)
        .bind(cmd.is_public)
        .bind(cmd.published_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn delete(&self, id: Uuid) -> Result<bool, DomainError> {
        let res = sqlx::query("DELETE FROM community_news WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }
}
