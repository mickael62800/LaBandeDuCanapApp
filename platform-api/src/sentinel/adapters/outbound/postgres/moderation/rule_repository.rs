use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::system::rule::Rule;
use platform_core::sentinel::domain::enums::moderation::flag_type::FlagType;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::moderation::rule_repository::RuleRepository;

pub struct PgRuleRepository {
    pool: PgPool,
}

impl PgRuleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct RuleRow {
    id: Uuid,
    guild_id: String,
    flag_type: String,
    weight: f64,
    threshold_warn: f64,
    threshold_delete: f64,
    threshold_mute: f64,
    threshold_ban: f64,
    enabled: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<RuleRow> for Rule {
    fn from(row: RuleRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id.into(),
            flag_type: FlagType::from_str_lossy(&row.flag_type),
            weight: row.weight,
            threshold_warn: row.threshold_warn,
            threshold_delete: row.threshold_delete,
            threshold_mute: row.threshold_mute,
            threshold_ban: row.threshold_ban,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl RuleRepository for PgRuleRepository {
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<Rule>, DomainError> {
        let rows = sqlx::query_as::<_, RuleRow>(
            "SELECT * FROM rules WHERE guild_id = $1 ORDER BY flag_type",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(Rule::from).collect())
    }

    async fn find_all(&self) -> Result<Vec<Rule>, DomainError> {
        let rows = sqlx::query_as::<_, RuleRow>("SELECT * FROM rules ORDER BY guild_id, flag_type")
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(rows.into_iter().map(Rule::from).collect())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Rule>, DomainError> {
        let row = sqlx::query_as::<_, RuleRow>("SELECT * FROM rules WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(row.map(Rule::from))
    }

    async fn toggle(&self, id: Uuid, enabled: bool) -> Result<(), DomainError> {
        let result = sqlx::query("UPDATE rules SET enabled = $1, updated_at = NOW() WHERE id = $2")
            .bind(enabled)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("Regle {}", id)));
        }

        Ok(())
    }

    async fn save(&self, rule: &Rule) -> Result<Rule, DomainError> {
        let row = sqlx::query_as::<_, RuleRow>(
            r#"
            INSERT INTO rules (id, guild_id, flag_type, weight, threshold_warn, threshold_delete, threshold_mute, threshold_ban, enabled, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (guild_id, flag_type) DO UPDATE SET
                weight = EXCLUDED.weight,
                threshold_warn = EXCLUDED.threshold_warn,
                threshold_delete = EXCLUDED.threshold_delete,
                threshold_mute = EXCLUDED.threshold_mute,
                threshold_ban = EXCLUDED.threshold_ban,
                enabled = EXCLUDED.enabled,
                updated_at = EXCLUDED.updated_at
            RETURNING *
            "#,
        )
        .bind(rule.id)
        .bind(rule.guild_id.as_str())
        .bind(rule.flag_type.as_str())
        .bind(rule.weight)
        .bind(rule.threshold_warn)
        .bind(rule.threshold_delete)
        .bind(rule.threshold_mute)
        .bind(rule.threshold_ban)
        .bind(rule.enabled)
        .bind(rule.created_at)
        .bind(rule.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(Rule::from(row))
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query("DELETE FROM rules WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("Regle {}", id)));
        }

        Ok(())
    }

    async fn seed_defaults(&self, rules: &[Rule]) -> Result<(), DomainError> {
        // Insertion idempotente ligne a ligne : les regles deja presentes
        // (guild_id, flag_type) ne sont pas ecrasees. Couvre les nouvelles
        // guilds + le retro-seed des anciennes au prochain startup du bot.
        for rule in rules {
            sqlx::query(
                "INSERT INTO rules (id, guild_id, flag_type, weight, threshold_warn, \
                 threshold_delete, threshold_mute, threshold_ban, enabled) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
                 ON CONFLICT (guild_id, flag_type) DO NOTHING",
            )
            .bind(rule.id)
            .bind(rule.guild_id.as_str())
            .bind(rule.flag_type.as_str())
            .bind(rule.weight)
            .bind(rule.threshold_warn)
            .bind(rule.threshold_delete)
            .bind(rule.threshold_mute)
            .bind(rule.threshold_ban)
            .bind(rule.enabled)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        }
        Ok(())
    }
}
