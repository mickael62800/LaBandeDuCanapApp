//! Adapter postgres du port `SecurityAuditRepository` : journal d'audit,
//! logins reussis et purge multi-tables des logs securite.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::pg_err_ctx;
use ops_core::domain::entities::security_audit::{
    AuditLogEntry, AuditLogFilter, CleanupOptions, CleanupReport, SuccessfulLogin,
};
use ops_core::domain::errors::DomainError;
use ops_core::ports::outbound::security_audit_repository::SecurityAuditRepository;

const TBL: &str = "audit_logs";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

/// DELETE avec ou sans filtre temporel sur la colonne donnee. Best-effort :
/// logge et renvoie 0 si la requete echoue (la purge globale continue).
async fn purge_table(pool: &PgPool, table: &str, ts_col: &str, days: i64) -> u64 {
    let sql = if days == 0 {
        format!("DELETE FROM {table}")
    } else {
        format!("DELETE FROM {table} WHERE {ts_col} < NOW() - INTERVAL '{days} days'")
    };
    sqlx::query(&sql)
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, table = table, "purge table");
            0
        })
}

pub struct PgSecurityAuditRepository {
    pool: PgPool,
}

impl PgSecurityAuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SecurityAuditRepository for PgSecurityAuditRepository {
    async fn list_audit_logs(
        &self,
        filter: AuditLogFilter,
    ) -> Result<Vec<AuditLogEntry>, DomainError> {
        // Construction dynamique safe : seuls les noms de colonnes sont
        // hardcodes, les valeurs sont bindees via $N.
        let mut sql = String::from(
            "SELECT id::text, guild_id, event_type, actor_id, actor_name, \
                    target_id, target_name, details, created_at \
             FROM audit_logs WHERE 1=1",
        );
        let mut idx = 1;
        if filter.guild_id.is_some() {
            sql.push_str(&format!(" AND guild_id = ${idx}"));
            idx += 1;
        }
        if filter.event_type_prefix.is_some() {
            sql.push_str(&format!(" AND event_type LIKE ${idx} || '%'"));
            idx += 1;
        }
        sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ${idx}"));

        let mut q = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                serde_json::Value,
                DateTime<Utc>,
            ),
        >(&sql);
        if let Some(g) = &filter.guild_id {
            q = q.bind(g);
        }
        if let Some(p) = &filter.event_type_prefix {
            q = q.bind(p);
        }
        q = q.bind(filter.limit);

        let rows = q.fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    guild_id,
                    event_type,
                    actor_id,
                    actor_name,
                    target_id,
                    target_name,
                    details,
                    created_at,
                )| {
                    AuditLogEntry {
                        id,
                        guild_id,
                        event_type,
                        actor_id,
                        actor_name,
                        target_id,
                        target_name,
                        details,
                        created_at,
                    }
                },
            )
            .collect())
    }

    async fn list_recent_logins(&self, limit: i64) -> Result<Vec<SuccessfulLogin>, DomainError> {
        let rows = sqlx::query_as::<
            _,
            (
                DateTime<Utc>,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
            ),
        >(
            "SELECT logged_at, discord_user_id, username, client_ip, user_agent \
             FROM successful_logins ORDER BY logged_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(
                |(timestamp, discord_user_id, username, client_ip, user_agent)| SuccessfulLogin {
                    timestamp,
                    discord_user_id,
                    username,
                    client_ip,
                    user_agent,
                },
            )
            .collect())
    }

    async fn cleanup(&self, options: CleanupOptions) -> Result<CleanupReport, DomainError> {
        let days = options.older_than_days.max(0);
        let mut report = CleanupReport::default();

        if options.include_api_logs {
            let sql = if days == 0 {
                "DELETE FROM logs WHERE category = 'api'".to_string()
            } else {
                format!(
                    "DELETE FROM logs WHERE category = 'api' AND timestamp < NOW() - INTERVAL '{days} days'"
                )
            };
            report.deleted_api_logs = sqlx::query(&sql)
                .execute(&self.pool)
                .await
                .map_err(pg_err)?
                .rows_affected();
        }
        if options.include_audit_logs {
            report.deleted_audit_logs =
                purge_table(&self.pool, "audit_logs", "created_at", days).await;
        }
        if options.include_server_events {
            report.deleted_server_events =
                purge_table(&self.pool, "server_events", "timestamp", days).await;
        }
        if options.include_successful_logins {
            report.deleted_successful_logins =
                purge_table(&self.pool, "successful_logins", "logged_at", days).await;
        }
        if options.include_manual_bans {
            report.deleted_manual_bans =
                purge_table(&self.pool, "manual_ip_bans", "banned_at", days).await;
        }

        Ok(report)
    }
}
