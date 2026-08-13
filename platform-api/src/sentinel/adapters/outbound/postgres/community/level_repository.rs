use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::level::UserLevel;
use platform_core::sentinel::domain::entities::community::level::XpSource;
use platform_core::sentinel::domain::entities::community::progression_calc::StreakState;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::level_repository::LevelRepository;

pub struct PgLevelRepository {
    pool: PgPool,
}

impl PgLevelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn materialized_view_unpopulated(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("55000")
    )
}

#[derive(sqlx::FromRow)]
struct UserLevelRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    username: String,
    xp: i64,
    level: i32,
    xp_text: i64,
    level_text: i32,
    xp_voice: i64,
    level_voice: i32,
    last_xp_at: chrono::DateTime<chrono::Utc>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<UserLevelRow> for UserLevel {
    fn from(r: UserLevelRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            user_id: r.user_id.into(),
            username: r.username,
            xp: r.xp,
            level: r.level,
            xp_text: r.xp_text,
            level_text: r.level_text,
            xp_voice: r.xp_voice,
            level_voice: r.level_voice,
            last_xp_at: r.last_xp_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl LevelRepository for PgLevelRepository {
    async fn get_user_level(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserLevel>, DomainError> {
        let row = sqlx::query_as::<_, UserLevelRow>(
            "SELECT id, guild_id, user_id, username, xp, level, xp_text, level_text, xp_voice, level_voice, last_xp_at, created_at, updated_at FROM user_levels WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.map(UserLevel::from))
    }

    async fn upsert_user_level(&self, user: &UserLevel) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO user_levels (id, guild_id, user_id, username, xp, level, xp_text, level_text, xp_voice, level_voice, last_xp_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
               ON CONFLICT (guild_id, user_id) DO UPDATE SET
                 username = $4, xp = $5, level = $6, xp_text = $7, level_text = $8,
                 xp_voice = $9, level_voice = $10, last_xp_at = $11, updated_at = NOW()"#,
        )
        .bind(user.id)
        .bind(user.guild_id.as_str())
        .bind(user.user_id.as_str())
        .bind(&user.username)
        .bind(user.xp)
        .bind(user.level)
        .bind(user.xp_text)
        .bind(user.level_text)
        .bind(user.xp_voice)
        .bind(user.level_voice)
        .bind(user.last_xp_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn add_xp_atomic(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        amount: i64,
        source: XpSource,
    ) -> Result<UserLevel, DomainError> {
        // Determine quelles colonnes incrementer selon la source.
        // SECURITE : xp_col est strictement controle par le match (jamais d'input utilisateur).
        let xp_col = match source {
            XpSource::Text => "xp_text",
            XpSource::Voice => "xp_voice",
        };

        let query = format!(
            r#"INSERT INTO user_levels (id, guild_id, user_id, username, xp, level, xp_text, level_text, xp_voice, level_voice, last_xp_at, updated_at)
               VALUES (gen_random_uuid(), $1, $2, $3, $4, 0,
                       CASE WHEN '{xp_col}' = 'xp_text' THEN $4 ELSE 0 END,
                       0,
                       CASE WHEN '{xp_col}' = 'xp_voice' THEN $4 ELSE 0 END,
                       0, NOW(), NOW())
               ON CONFLICT (guild_id, user_id) DO UPDATE SET
                 username = $3,
                 xp = user_levels.xp + $4,
                 {xp_col} = user_levels.{xp_col} + $4,
                 last_xp_at = NOW(),
                 updated_at = NOW()
               RETURNING id, guild_id, user_id, username, xp, level, xp_text, level_text, xp_voice, level_voice, last_xp_at, created_at, updated_at"#,
            xp_col = xp_col
        );

        let row = sqlx::query_as::<_, UserLevelRow>(&query)
            .bind(guild_id)
            .bind(user_id)
            .bind(username)
            .bind(amount)
            .fetch_one(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(UserLevel::from(row))
    }

    async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError> {
        // Phase 2 A.2 — Lit depuis `mv_level_leaderboard` (5 min staleness max).
        let rows = sqlx::query_as::<_, UserLevelRow>(
            "SELECT id, guild_id, user_id, username, xp, level, xp_text, level_text, xp_voice, level_voice, last_xp_at, created_at, updated_at FROM mv_level_leaderboard WHERE guild_id = $1 ORDER BY rank LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await;

        let rows = match rows {
            Ok(rows) => rows,
            Err(error) if materialized_view_unpopulated(&error) => {
                // Protection pendant le bootstrap : la table source possede
                // deja l'index (guild_id, xp DESC), ce repli reste donc borne
                // et evite un 500 pendant le premier refresh de la vue.
                warn!(
                    guild_id,
                    "Leaderboard materialized view is not populated; using indexed source table"
                );
                sqlx::query_as::<_, UserLevelRow>(
                    "SELECT id, guild_id, user_id, username, xp, level, xp_text, level_text, xp_voice, level_voice, last_xp_at, created_at, updated_at FROM user_levels WHERE guild_id = $1 ORDER BY xp DESC LIMIT $2",
                )
                .bind(guild_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
                .map_err(pg_err)?
            }
            Err(error) => return Err(pg_err(error)),
        };

        Ok(rows.into_iter().map(UserLevel::from).collect())
    }

    async fn get_leaderboard_by_source(
        &self,
        guild_id: &str,
        source: XpSource,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError> {
        let order_col = match source {
            XpSource::Text => "xp_text",
            XpSource::Voice => "xp_voice",
        };
        let query = format!(
            "SELECT id, guild_id, user_id, username, xp, level, xp_text, level_text, xp_voice, level_voice, last_xp_at, created_at, updated_at FROM user_levels WHERE guild_id = $1 ORDER BY {} DESC LIMIT $2",
            order_col
        );
        let rows = sqlx::query_as::<_, UserLevelRow>(&query)
            .bind(guild_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(rows.into_iter().map(UserLevel::from).collect())
    }

    async fn refresh_leaderboard_view(&self) -> Result<(), DomainError> {
        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY mv_level_leaderboard")
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn get_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<StreakState>, DomainError> {
        let row: Option<(i32, i32, i32, i32)> = sqlx::query_as(
            "SELECT streak_current, streak_best, streak_last_day, streak_last_year \
             FROM user_levels WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.map(|(current, best, last_day, last_year)| StreakState {
            current: current.max(0) as u32,
            best: best.max(0) as u32,
            last_day: last_day.max(0) as u32,
            last_year,
        }))
    }

    async fn update_streak(
        &self,
        guild_id: &str,
        user_id: &str,
        state: StreakState,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE user_levels SET streak_current = $3, streak_best = $4, \
             streak_last_day = $5, streak_last_year = $6, updated_at = NOW() \
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(state.current as i32)
        .bind(state.best as i32)
        .bind(state.last_day as i32)
        .bind(state.last_year)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
