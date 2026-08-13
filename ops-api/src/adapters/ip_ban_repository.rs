//! Adapter postgres du port `IpBanRepository` (table `manual_ip_bans` + purge
//! des logs API). Pas de logique metier ici : juste le SQL.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::pg_err_ctx;
use ops_core::domain::entities::ip_ban::ManualIpBan;
use ops_core::domain::errors::DomainError;
use ops_core::ports::outbound::ip_ban_repository::IpBanRepository;

const TBL: &str = "manual_ip_bans";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgIpBanRepository {
    pool: PgPool,
}

impl PgIpBanRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IpBanRepository for PgIpBanRepository {
    async fn record_manual_ban(
        &self,
        ip: &str,
        banned_by: &str,
        reason: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO manual_ip_bans (ip, banned_at, banned_by, reason, unbanned_at, unbanned_by) \
             VALUES ($1, NOW(), $2, $3, NULL, NULL) \
             ON CONFLICT (ip) DO UPDATE SET banned_at = NOW(), banned_by = EXCLUDED.banned_by, \
                reason = EXCLUDED.reason, unbanned_at = NULL, unbanned_by = NULL",
        )
        .bind(ip)
        .bind(banned_by)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn mark_unbanned(&self, ip: &str, unbanned_by: &str) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE manual_ip_bans SET unbanned_at = NOW(), unbanned_by = $2 \
             WHERE ip = $1 AND unbanned_at IS NULL",
        )
        .bind(ip)
        .bind(unbanned_by)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_active(&self) -> Result<Vec<ManualIpBan>, DomainError> {
        let rows: Vec<(String, DateTime<Utc>, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT ip, banned_at, banned_by, reason \
             FROM manual_ip_bans WHERE unbanned_at IS NULL ORDER BY banned_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|(ip, banned_at, banned_by, reason)| ManualIpBan {
                ip,
                banned_at,
                banned_by,
                reason,
            })
            .collect())
    }
}
