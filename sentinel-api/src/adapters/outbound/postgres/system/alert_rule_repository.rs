//! Adapter sortant Postgres des règles d'alerte (`alert_rules`).
//! Tout le SQL du domaine vit ici.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::adapters::outbound::postgres::pg_err;
use ops_core::domain::entities::alert_rule::{AlertRule, AlertRuleUpdate};
use sentinel_core::domain::errors::DomainError;
use ops_core::ports::outbound::alert_rule_repository::AlertRuleRepository;

pub struct PgAlertRuleRepository {
    pool: PgPool,
}

impl PgAlertRuleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AlertRuleRow {
    id: String,
    label: String,
    metric: String,
    comparator: String,
    threshold: Option<f64>,
    enabled: bool,
    severity: String,
    cooldown_secs: i32,
}

impl From<AlertRuleRow> for AlertRule {
    fn from(r: AlertRuleRow) -> Self {
        AlertRule {
            id: r.id,
            label: r.label,
            metric: r.metric,
            comparator: r.comparator,
            threshold: r.threshold,
            enabled: r.enabled,
            severity: r.severity,
            cooldown_secs: r.cooldown_secs,
        }
    }
}

const SELECT_COLS: &str =
    "id, label, metric, comparator, threshold, enabled, severity, cooldown_secs";

#[async_trait]
impl AlertRuleRepository for PgAlertRuleRepository {
    async fn list(&self) -> Result<Vec<AlertRule>, DomainError> {
        let rows: Vec<AlertRuleRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLS} FROM alert_rules ORDER BY id"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(
        &self,
        id: &str,
        update: &AlertRuleUpdate,
    ) -> Result<Option<AlertRule>, DomainError> {
        // COALESCE : seuls les champs fournis sont modifies. threshold n'est pas
        // remis a NULL par cet endpoint (les metriques booleennes le gardent NULL).
        let row: Option<AlertRuleRow> = sqlx::query_as(&format!(
            "UPDATE alert_rules SET \
             enabled = COALESCE($2, enabled), \
             threshold = COALESCE($3, threshold), \
             severity = COALESCE($4, severity), \
             cooldown_secs = COALESCE($5, cooldown_secs), \
             updated_at = NOW() \
             WHERE id = $1 RETURNING {SELECT_COLS}"
        ))
        .bind(id)
        .bind(update.enabled)
        .bind(update.threshold)
        .bind(&update.severity)
        .bind(update.cooldown_secs)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }
}
