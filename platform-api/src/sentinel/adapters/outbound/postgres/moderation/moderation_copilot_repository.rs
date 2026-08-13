//! Adapter Postgres du port `ModerationCopilotRepository` (lecture seule).
//!
//! - Historique de sanctions : agrege depuis `audit_logs` (event_type `mod_%`),
//!   source de verite depuis la Phase 4 (voir `moderation_repository`).
//! - Jurisprudence : agrege depuis `automod_reviews`. CRITIQUE — anti-ancrage :
//!   toutes les requetes de jurisprudence EXCLUENT `status = 'voting'` (seules
//!   les reviews deja tranchees comptent). Prefere `applied_action` puis
//!   `decided_action`. Les categories de flag sont les cles booleennes du
//!   JSONB `flags` (`spam`, `insult`, `link`, `phishing`, ...).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;

use crate::sentinel::adapters::outbound::postgres::pg_ctx;
use platform_core::sentinel::domain::entities::moderation::copilot::PrecedentDistribution;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::moderation::moderation_copilot_repository::ModerationCopilotRepository;

pub struct PgModerationCopilotRepository {
    pool: PgPool,
}

impl PgModerationCopilotRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ModerationCopilotRepository for PgModerationCopilotRepository {
    async fn count_sanctions_by_type(
        &self,
        guild_id: &str,
        user_id: &str,
        since: DateTime<Utc>,
    ) -> Result<Vec<(String, u32)>, DomainError> {
        // action_type = event_type sans le prefixe 'mod_'.
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT substring(event_type FROM 5) AS action_type, COUNT(*) AS c \
             FROM audit_logs \
             WHERE guild_id = $1 AND target_id = $2 \
               AND event_type LIKE 'mod_%' AND created_at >= $3 \
             GROUP BY action_type \
             ORDER BY c DESC, action_type",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("copilot_count_sanctions_by_type"))?;

        Ok(rows
            .into_iter()
            .map(|(action, count)| (action, count.max(0) as u32))
            .collect())
    }

    async fn last_sanction_at(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        let row: Option<Option<DateTime<Utc>>> = sqlx::query_scalar(
            "SELECT MAX(created_at) FROM audit_logs \
             WHERE guild_id = $1 AND target_id = $2 AND event_type LIKE 'mod_%'",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("copilot_last_sanction_at"))?;

        Ok(row.flatten())
    }

    async fn count_open_reviews(&self, guild_id: &str, user_id: &str) -> Result<u32, DomainError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM automod_reviews \
             WHERE guild_id = $1 AND user_id = $2 \
               AND status IN ('voting','pending','decided')",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_ctx("copilot_count_open_reviews"))?;

        Ok(count.max(0) as u32)
    }

    async fn dominant_flag_category(
        &self,
        guild_id: &str,
        user_id: &str,
        since: DateTime<Utc>,
    ) -> Result<Option<String>, DomainError> {
        // Compte chaque cle de flag a `true` parmi les reviews TRANCHEES du
        // membre (anti-ancrage : status <> 'voting'). Retourne la plus frequente.
        let row: Option<String> = sqlx::query_scalar(
            "SELECT f.key \
             FROM automod_reviews r, jsonb_each(r.flags) AS f(key, value) \
             WHERE r.guild_id = $1 AND r.user_id = $2 \
               AND r.status <> 'voting' AND r.created_at >= $3 \
               AND f.value = 'true'::jsonb \
             GROUP BY f.key \
             ORDER BY COUNT(*) DESC, f.key \
             LIMIT 1",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(since)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("copilot_dominant_flag_category"))?;

        Ok(row)
    }

    async fn aggregate_decided_by_flag(
        &self,
        guild_id: &str,
        flag_category: &str,
        since: DateTime<Utc>,
    ) -> Result<PrecedentDistribution, DomainError> {
        // Distribution des actions retenues (applied_action prioritaire, sinon
        // decided_action) sur les reviews de la guild portant ce flag a `true`.
        // ANTI-ANCRAGE : status <> 'voting' — seules les decisions tranchees.
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT COALESCE(applied_action, decided_action) AS action, COUNT(*) AS c \
             FROM automod_reviews \
             WHERE guild_id = $1 \
               AND status <> 'voting' \
               AND created_at >= $3 \
               AND (flags -> $2) = 'true'::jsonb \
               AND COALESCE(applied_action, decided_action) IS NOT NULL \
             GROUP BY action \
             ORDER BY c DESC, action",
        )
        .bind(guild_id)
        .bind(flag_category)
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("copilot_aggregate_decided_by_flag"))?;

        let counts_by_action: Vec<(String, u32)> = rows
            .into_iter()
            .map(|(action, count)| (action, count.max(0) as u32))
            .collect();
        let total = counts_by_action.iter().map(|(_, c)| *c).sum();

        Ok(PrecedentDistribution {
            flag_category: flag_category.to_string(),
            counts_by_action,
            total,
        })
    }
}
