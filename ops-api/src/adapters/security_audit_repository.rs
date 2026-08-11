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

/// DELETE (filtre temporel + condition optionnelle) DANS une transaction.
///
/// Contrairement a l'ancienne version best-effort qui renvoyait 0 en cas
/// d'erreur — rendant une panne indistinguable d'une table vide — l'erreur est
/// propagee : la transaction appelante annule alors toutes les suppressions.
async fn purge_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    table: &str,
    ts_col: &str,
    days: i64,
    extra_where: Option<&str>,
) -> Result<u64, DomainError> {
    let mut sql = format!("DELETE FROM {table} WHERE 1=1");
    if let Some(cond) = extra_where {
        sql.push_str(&format!(" AND {cond}"));
    }
    if days > 0 {
        sql.push_str(&format!(" AND {ts_col} < NOW() - INTERVAL '{days} days'"));
    }
    sqlx::query(&sql)
        .execute(&mut **tx)
        .await
        .map(|r| r.rows_affected())
        .map_err(pg_err)
}

pub struct PgSecurityAuditRepository {
    pool: PgPool,
    /// Le journal des logins n'est plus dans cette base : il appartient a
    /// l'identite. Ce repository reste « Pg » pour l'audit et la purge des
    /// autres tables, et delegue pour celle-la.
    auth: crate::adapters::auth_logins::AuthLoginsClient,
}

impl PgSecurityAuditRepository {
    pub fn new(pool: PgPool, auth: crate::adapters::auth_logins::AuthLoginsClient) -> Self {
        Self { pool, auth }
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
             FROM ops_audit_logs_v WHERE 1=1",
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
        self.auth.recent(limit).await
    }

    async fn cleanup(&self, options: CleanupOptions) -> Result<CleanupReport, DomainError> {
        // 1. Validation en amont : rien n'est supprime avant ce point. Sans
        //    cible selectionnee, c'est un no-op (on n'ouvre meme pas de
        //    transaction).
        let days = options.older_than_days.max(0);
        let touches_local = options.include_api_logs
            || options.include_audit_logs
            || options.include_server_events
            || options.include_manual_bans;

        let mut report = CleanupReport::default();

        // 2. Suppressions LOCALES dans une seule transaction : une erreur en
        //    cours de route annule tout, au lieu de laisser une purge
        //    partiellement appliquee et un compteur trompeur.
        if touches_local {
            let mut tx = self.pool.begin().await.map_err(pg_err)?;
            if options.include_api_logs {
                report.deleted_api_logs =
                    purge_in_tx(&mut tx, "ops_logs_v", "timestamp", days, Some("category = 'api'"))
                        .await?;
            }
            if options.include_audit_logs {
                report.deleted_audit_logs =
                    purge_in_tx(&mut tx, "ops_audit_logs_v", "created_at", days, None).await?;
            }
            if options.include_server_events {
                report.deleted_server_events =
                    purge_in_tx(&mut tx, "server_events", "timestamp", days, None).await?;
            }
            if options.include_manual_bans {
                report.deleted_manual_bans =
                    purge_in_tx(&mut tx, "manual_ip_bans", "banned_at", days, None).await?;
            }
            tx.commit().await.map_err(pg_err)?;
        }

        // 3. Purge DISTANTE (journal des logins, heberge par l'identite) : hors
        //    transaction locale — elle ne peut pas participer au commit Postgres
        //    et ne doit surtout pas annuler ce qui vient d'etre valide ici.
        //    Reste best-effort (cf. `AuthLoginsClient::purge`).
        if options.include_successful_logins {
            report.deleted_successful_logins = self.auth.purge(days).await;
        }

        Ok(report)
    }
}
