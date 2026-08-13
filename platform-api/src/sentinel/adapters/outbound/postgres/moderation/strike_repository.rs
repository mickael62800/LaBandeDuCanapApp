use crate::sentinel::adapters::outbound::postgres::pg_ctx;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::moderation::action::strikes::StrikeConfig;
use platform_core::sentinel::domain::entities::moderation::action::strikes::StrikeThreshold;
use platform_core::sentinel::domain::entities::moderation::action::strikes::UserStrike;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::moderation::strike_repository::StrikeRepository;

pub struct PgStrikeRepository {
    pool: PgPool,
}

impl PgStrikeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct StrikeRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    reason: String,
    source: String,
    infraction_id: Option<Uuid>,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<StrikeRow> for UserStrike {
    fn from(r: StrikeRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            user_id: r.user_id.into(),
            reason: r.reason,
            source: r.source,
            infraction_id: r.infraction_id,
            expires_at: r.expires_at,
            created_at: r.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StrikeConfigRow {
    guild_id: String,
    window_secs: i64,
    thresholds: serde_json::Value,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<StrikeConfigRow> for StrikeConfig {
    fn from(r: StrikeConfigRow) -> Self {
        let thresholds: Vec<StrikeThreshold> = match serde_json::from_value(r.thresholds.clone()) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(guild_id = %r.guild_id, error = %e, raw = %r.thresholds, "Parse thresholds JSON echoue, fallback vec![]");
                Vec::new()
            }
        };
        Self {
            guild_id: r.guild_id.into(),
            window_secs: r.window_secs,
            thresholds,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl StrikeRepository for PgStrikeRepository {
    async fn save_strike(&self, strike: &UserStrike) -> Result<(), DomainError> {
        // ON CONFLICT (F4) : idempotence -> un seul strike par action de
        // moderation (infraction_id). Un re-appel pour la meme action ne cree
        // pas de second strike (pas de double escalade).
        sqlx::query(
            "INSERT INTO user_strikes (id, guild_id, user_id, reason, source, infraction_id, expires_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (infraction_id) WHERE infraction_id IS NOT NULL DO NOTHING"
        )
        .bind(strike.id)
        .bind(strike.guild_id.as_str())
        .bind(strike.user_id.as_str())
        .bind(&strike.reason)
        .bind(&strike.source)
        .bind(strike.infraction_id)
        .bind(strike.expires_at)
        .bind(strike.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("save_strike"))?;
        Ok(())
    }

    async fn find_active_strikes(
        &self,
        guild_id: &str,
        user_id: &str,
        window_secs: i64,
    ) -> Result<Vec<UserStrike>, DomainError> {
        let cutoff = Utc::now() - Duration::seconds(window_secs);
        // window_secs <= 0 = strikes "permanents" : on ne filtre PAS par
        // created_at (sinon cutoff = maintenant -> 0 strike compte -> escalade
        // morte). L'expiration reste geree par expires_at (NULL = permanent).
        let no_window = window_secs <= 0;
        let rows = sqlx::query_as::<_, StrikeRow>(
            "SELECT id, guild_id, user_id, reason, source, infraction_id, expires_at, created_at
             FROM user_strikes
             WHERE guild_id = $1 AND user_id = $2
               AND (expires_at IS NULL OR expires_at > NOW())
               AND (created_at > $3 OR $4)
             ORDER BY created_at DESC",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(cutoff)
        .bind(no_window)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("find_active_strikes"))?;

        Ok(rows.into_iter().map(UserStrike::from).collect())
    }

    async fn delete_strikes(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM user_strikes WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("delete_strikes"))?;
        Ok(())
    }

    async fn delete_strike_by_infraction_id(
        &self,
        infraction_id: Uuid,
    ) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM user_strikes WHERE infraction_id = $1")
            .bind(infraction_id)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("delete_strike_by_infraction_id"))?;
        Ok(result.rows_affected())
    }

    async fn get_config(&self, guild_id: &str) -> Result<Option<StrikeConfig>, DomainError> {
        let row = sqlx::query_as::<_, StrikeConfigRow>(
            "SELECT guild_id, window_secs, thresholds, enabled, created_at, updated_at
             FROM strike_config WHERE guild_id = $1",
        )
        .bind(guild_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("get_strike_config"))?;

        Ok(row.map(StrikeConfig::from))
    }

    async fn save_config(&self, config: &StrikeConfig) -> Result<(), DomainError> {
        let thresholds_json = serde_json::to_value(&config.thresholds)
            .map_err(|e| DomainError::Internal(format!("serialize thresholds: {e}")))?;

        sqlx::query(
            "INSERT INTO strike_config (guild_id, window_secs, thresholds, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, NOW(), NOW())
             ON CONFLICT (guild_id) DO UPDATE SET
               window_secs = EXCLUDED.window_secs,
               thresholds = EXCLUDED.thresholds,
               enabled = EXCLUDED.enabled,
               updated_at = NOW()"
        )
        .bind(config.guild_id.as_str())
        .bind(config.window_secs)
        .bind(thresholds_json)
        .bind(config.enabled)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("save_strike_config"))?;

        Ok(())
    }
}
