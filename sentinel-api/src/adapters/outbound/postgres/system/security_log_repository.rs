//! Adapter postgres du port `SecurityLogRepository` : agregations sur la table
//! `logs` (categorie `api`). Le mapping fenetre -> intervalle SQL vit ici.
//!
//! Les valeurs interpolees dans le SQL (`interval`, `bucket`) proviennent
//! d'un enum domaine et d'un entier borne par le use case : pas d'injection.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::super::pg_err_ctx;
use ops_core::domain::entities::security_log::{
    AuthFailure, LogWindow, TopIp, TrafficPoint,
};
use sentinel_core::domain::errors::DomainError;
use ops_core::ports::outbound::security_log_repository::SecurityLogRepository;

const TBL: &str = "logs";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

fn interval(window: LogWindow) -> &'static str {
    match window {
        LogWindow::OneHour => "1 hour",
        LogWindow::TwentyFourHours => "24 hours",
        LogWindow::SevenDays => "7 days",
    }
}

pub struct PgSecurityLogRepository {
    pool: PgPool,
}

impl PgSecurityLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SecurityLogRepository for PgSecurityLogRepository {
    async fn top_ips(&self, window: LogWindow, limit: i64) -> Result<Vec<TopIp>, DomainError> {
        let interval = interval(window);
        let sql = format!(
            "SELECT \
                COALESCE(details->>'client_ip', '-') AS ip, \
                COUNT(*)::bigint AS total, \
                SUM(CASE WHEN level IN ('warn', 'error') THEN 1 ELSE 0 END)::bigint AS failed, \
                MAX(timestamp) AS last_seen \
             FROM logs \
             WHERE category = 'api' \
               AND timestamp > NOW() - INTERVAL '{interval}' \
               AND details->>'client_ip' IS NOT NULL \
               AND details->>'client_ip' != '-' \
             GROUP BY ip \
             ORDER BY total DESC \
             LIMIT {limit}"
        );
        let rows = sqlx::query_as::<_, (String, i64, i64, DateTime<Utc>)>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|(client_ip, total, failed, last_seen)| TopIp {
                client_ip,
                total,
                failed,
                last_seen,
            })
            .collect())
    }

    async fn auth_failures(
        &self,
        window: LogWindow,
        limit: i64,
    ) -> Result<Vec<AuthFailure>, DomainError> {
        let interval = interval(window);
        let sql = format!(
            "SELECT \
                timestamp, \
                COALESCE((details->>'status_code')::bigint, 0) AS status, \
                COALESCE(details->>'method', '?') AS method, \
                COALESCE(details->>'route', '?') AS route, \
                COALESCE(details->>'client_ip', '-') AS ip, \
                COALESCE(details->>'user_agent', '') AS ua \
             FROM logs \
             WHERE category = 'api' \
               AND timestamp > NOW() - INTERVAL '{interval}' \
               AND (details->>'status_code')::int IN (401, 403) \
             ORDER BY timestamp DESC \
             LIMIT {limit}"
        );
        let rows = sqlx::query_as::<_, (DateTime<Utc>, i64, String, String, String, String)>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(
                |(timestamp, status_code, method, route, client_ip, user_agent)| AuthFailure {
                    timestamp,
                    status_code,
                    method,
                    route,
                    client_ip,
                    user_agent,
                },
            )
            .collect())
    }

    async fn traffic_points(
        &self,
        window: LogWindow,
        bucket_minutes: i64,
    ) -> Result<Vec<TrafficPoint>, DomainError> {
        let interval = interval(window);
        let sql = format!(
            "SELECT \
                date_trunc('hour', timestamp) + \
                    INTERVAL '{bucket_minutes} min' * \
                    FLOOR(EXTRACT(MINUTE FROM timestamp) / {bucket_minutes}) AS bucket, \
                COUNT(*)::bigint AS total, \
                SUM(CASE WHEN level IN ('warn', 'error') THEN 1 ELSE 0 END)::bigint AS errors \
             FROM logs \
             WHERE category = 'api' \
               AND timestamp > NOW() - INTERVAL '{interval}' \
             GROUP BY bucket \
             ORDER BY bucket ASC"
        );
        let rows = sqlx::query_as::<_, (DateTime<Utc>, i64, i64)>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|(timestamp, total, errors)| TrafficPoint {
                timestamp,
                total,
                errors,
            })
            .collect())
    }
}
