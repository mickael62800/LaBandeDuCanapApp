//! Adapter sortant Postgres du classement mensuel : deltas d'XP (via baseline
//! `user_levels_monthly_snapshot`) + gestion des baselines. Tout le SQL du
//! domaine "classement mensuel" vit ici.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::sentinel::adapters::outbound::postgres::pg_ctx;
use platform_core::sentinel::domain::entities::community::monthly_ranking::RankingRow;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::monthly_ranking_repository::MonthlyRankingRepository;

pub struct PgMonthlyRankingRepository {
    pool: PgPool,
}

impl PgMonthlyRankingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MonthlyRankingRepository for PgMonthlyRankingRepository {
    async fn ranking_deltas(
        &self,
        guild_id: &str,
        baseline_period_ym: &str,
        excluded_roles: &[String],
    ) -> Result<Vec<RankingRow>, DomainError> {
        let rows: Vec<(String, i64, i64)> = sqlx::query_as(
            r#"SELECT ul.user_id,
                      (ul.xp_text  - COALESCE(s.xp_text, 0))  AS d_text,
                      (ul.xp_voice - COALESCE(s.xp_voice, 0)) AS d_voice
               FROM user_levels ul
               LEFT JOIN user_levels_monthly_snapshot s
                 ON s.guild_id = ul.guild_id AND s.user_id = ul.user_id AND s.period_ym = $2
               WHERE ul.guild_id = $1
                 AND NOT EXISTS (
                   SELECT 1 FROM guild_members gm
                   WHERE gm.guild_id = ul.guild_id
                     AND gm.user_id = ul.user_id
                     AND gm.roles ?| $3::text[]
                 )"#,
        )
        .bind(guild_id)
        .bind(baseline_period_ym)
        .bind(excluded_roles)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("monthly_ranking ranking_deltas"))?;

        Ok(rows
            .into_iter()
            .map(|(user_id, d_text, d_voice)| RankingRow {
                user_id,
                d_text,
                d_voice,
            })
            .collect())
    }

    async fn has_baseline(&self, guild_id: &str, period_ym: &str) -> Result<bool, DomainError> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM user_levels_monthly_snapshot WHERE guild_id = $1 AND period_ym = $2)",
        )
        .bind(guild_id)
        .bind(period_ym)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_ctx("monthly_ranking has_baseline"))
    }

    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT guild_id FROM guilds ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(pg_ctx("monthly_ranking list_guild_ids"))?;
        Ok(rows.into_iter().map(|(g,)| g).collect())
    }

    async fn baseline_partial_flag(
        &self,
        guild_id: &str,
        period_ym: &str,
    ) -> Result<Option<bool>, DomainError> {
        sqlx::query_scalar(
            "SELECT bool_or(partial) FROM user_levels_monthly_snapshot WHERE guild_id = $1 AND period_ym = $2",
        )
        .bind(guild_id)
        .bind(period_ym)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_ctx("monthly_ranking baseline_partial_flag"))
    }

    async fn has_prior_baseline(
        &self,
        guild_id: &str,
        period_ym: &str,
    ) -> Result<bool, DomainError> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM user_levels_monthly_snapshot WHERE guild_id = $1 AND period_ym < $2)",
        )
        .bind(guild_id)
        .bind(period_ym)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_ctx("monthly_ranking has_prior_baseline"))
    }

    async fn insert_baseline(
        &self,
        guild_id: &str,
        period_ym: &str,
        partial: bool,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO user_levels_monthly_snapshot (guild_id, user_id, period_ym, xp_text, xp_voice, partial)
               SELECT guild_id, user_id, $2, xp_text, xp_voice, $3
               FROM user_levels WHERE guild_id = $1
               ON CONFLICT (guild_id, user_id, period_ym) DO NOTHING"#,
        )
        .bind(guild_id)
        .bind(period_ym)
        .bind(partial)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("monthly_ranking insert_baseline"))?;
        Ok(())
    }
}
