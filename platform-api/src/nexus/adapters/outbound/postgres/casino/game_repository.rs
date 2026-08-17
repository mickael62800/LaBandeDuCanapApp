use async_trait::async_trait;
use sqlx::PgPool;

use super::super::pg_err;
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::casino::game_repository::Game;
use platform_core::nexus::ports::outbound::casino::game_repository::GamePanel;
use platform_core::nexus::ports::outbound::casino::game_repository::GameRepository;

pub struct PgGameRepository {
    pool: PgPool,
}

impl PgGameRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct GameRow {
    id: String,
    guild_id: String,
    game_name: String,
    created_by: String,
    created_at: String,
    emoji: Option<String>,
    category: Option<String>,
    role_id: Option<String>,
}

impl From<GameRow> for Game {
    fn from(r: GameRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            game_name: r.game_name,
            created_by: r.created_by,
            created_at: r.created_at,
            emoji: r.emoji,
            category: r.category,
            role_id: r.role_id,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PanelRow {
    id: String,
    guild_id: String,
    channel_id: String,
    message_id: String,
    category: Option<String>,
}

impl From<PanelRow> for GamePanel {
    fn from(r: PanelRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            channel_id: r.channel_id.into(),
            message_id: r.message_id.into(),
            category: r.category,
        }
    }
}

const GAME_COLS: &str =
    "id::text, guild_id, game_name, created_by, created_at::text, emoji, category, role_id";

#[async_trait]
impl GameRepository for PgGameRepository {
    async fn list(&self, guild_id: &str) -> Result<Vec<Game>, DomainError> {
        let sql = format!("SELECT {GAME_COLS} FROM games WHERE guild_id = $1 ORDER BY game_name");
        let rows: Vec<GameRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_by_category(
        &self,
        guild_id: &str,
        category: Option<&str>,
    ) -> Result<Vec<Game>, DomainError> {
        let rows: Vec<GameRow> = match category {
            Some(cat) => {
                let sql = format!(
                    "SELECT {GAME_COLS} FROM games WHERE guild_id = $1 AND LOWER(category) = LOWER($2) ORDER BY game_name"
                );
                sqlx::query_as(&sql)
                    .bind(guild_id)
                    .bind(cat)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(pg_err)?
            }
            None => {
                // Sans filtre, un panneau regroupe tout le catalogue du
                // serveur. C'est aussi la semantique exposee par le Web et
                // par `/game-admin panel` lorsque `category` est omise.
                let sql =
                    format!("SELECT {GAME_COLS} FROM games WHERE guild_id = $1 ORDER BY game_name");
                sqlx::query_as(&sql)
                    .bind(guild_id)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(pg_err)?
            }
        };
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn create(
        &self,
        guild_id: &str,
        game_name: &str,
        created_by: &str,
        emoji: Option<&str>,
        category: Option<&str>,
        role_id: Option<&str>,
    ) -> Result<Game, DomainError> {
        let sql = format!(
            "INSERT INTO games (guild_id, game_name, created_by, emoji, category, role_id) VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING {GAME_COLS}"
        );
        let row: GameRow = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(game_name)
            .bind(created_by)
            .bind(emoji)
            .bind(category)
            .bind(role_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("idx_games_guild_name") {
                    DomainError::Conflict("Un jeu avec ce nom existe deja".into())
                } else {
                    pg_err(e)
                }
            })?;
        Ok(row.into())
    }

    async fn update(
        &self,
        guild_id: &str,
        game_id: &str,
        game_name: Option<&str>,
        emoji: Option<Option<&str>>,
        category: Option<Option<&str>>,
    ) -> Result<Option<Game>, DomainError> {
        let update_name = game_name.is_some();
        let update_emoji = emoji.is_some();
        let update_category = category.is_some();
        if !update_name && !update_emoji && !update_category {
            let sql =
                format!("SELECT {GAME_COLS} FROM games WHERE guild_id = $1 AND id = $2::uuid");
            let row: Option<GameRow> = sqlx::query_as(&sql)
                .bind(guild_id)
                .bind(game_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;
            return Ok(row.map(Into::into));
        }
        let sql = format!(
            "UPDATE games SET \
                game_name = CASE WHEN $3::bool THEN $4 ELSE game_name END, \
                emoji = CASE WHEN $5::bool THEN $6 ELSE emoji END, \
                category = CASE WHEN $7::bool THEN $8 ELSE category END \
             WHERE guild_id = $1 AND id = $2::uuid \
             RETURNING {GAME_COLS}"
        );
        let row: Option<GameRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(game_id)
            .bind(update_name)
            .bind(game_name.unwrap_or(""))
            .bind(update_emoji)
            .bind(emoji.flatten())
            .bind(update_category)
            .bind(category.flatten())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("idx_games_guild_name") {
                    DomainError::Conflict("Un jeu avec ce nom existe deja".into())
                } else {
                    pg_err(e)
                }
            })?;
        Ok(row.map(Into::into))
    }

    async fn delete(&self, guild_id: &str, game_id: &str) -> Result<bool, DomainError> {
        let res = sqlx::query("DELETE FROM games WHERE guild_id = $1 AND id = $2::uuid")
            .bind(guild_id)
            .bind(game_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn find_by_name(
        &self,
        guild_id: &str,
        game_name: &str,
    ) -> Result<Option<Game>, DomainError> {
        let sql = format!(
            "SELECT {GAME_COLS} FROM games WHERE guild_id = $1 AND LOWER(game_name) = LOWER($2)"
        );
        let row: Option<GameRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(game_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn set_role_id(
        &self,
        guild_id: &str,
        game_id: &str,
        role_id: Option<&str>,
    ) -> Result<Option<Game>, DomainError> {
        let sql = format!(
            "UPDATE games SET role_id = $3 WHERE guild_id = $1 AND id = $2::uuid RETURNING {GAME_COLS}"
        );
        let row: Option<GameRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(game_id)
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn save_panel(
        &self,
        guild_id: &str,
        channel_id: &str,
        message_id: &str,
        category: Option<&str>,
    ) -> Result<GamePanel, DomainError> {
        let row: PanelRow = sqlx::query_as(
            "INSERT INTO game_panels (guild_id, channel_id, message_id, category) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (guild_id, COALESCE(category, '')) DO UPDATE SET \
               channel_id = EXCLUDED.channel_id, \
               message_id = EXCLUDED.message_id \
             RETURNING id::text, guild_id, channel_id, message_id, category",
        )
        .bind(guild_id)
        .bind(channel_id)
        .bind(message_id)
        .bind(category)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.into())
    }

    async fn find_panel_by_message(
        &self,
        guild_id: &str,
        message_id: &str,
    ) -> Result<Option<GamePanel>, DomainError> {
        let row: Option<PanelRow> = sqlx::query_as(
            "SELECT id::text, guild_id, channel_id, message_id, category FROM game_panels WHERE guild_id = $1 AND message_id = $2",
        )
        .bind(guild_id).bind(message_id)
        .fetch_optional(&self.pool).await.map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn list_panels(&self, guild_id: &str) -> Result<Vec<GamePanel>, DomainError> {
        let rows: Vec<PanelRow> = sqlx::query_as(
            "SELECT id::text, guild_id, channel_id, message_id, category FROM game_panels WHERE guild_id = $1 ORDER BY category NULLS FIRST",
        )
        .bind(guild_id)
        .fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn delete_panel(&self, guild_id: &str, message_id: &str) -> Result<bool, DomainError> {
        let result = sqlx::query("DELETE FROM game_panels WHERE guild_id = $1 AND message_id = $2")
            .bind(guild_id)
            .bind(message_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(result.rows_affected() > 0)
    }
}
