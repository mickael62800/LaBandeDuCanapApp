use crate::sentinel::adapters::outbound::postgres::pg_ctx;
use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::audit::audit_log::AuditLog;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::audit::manage_audit_logs::AuditLogFilters;
use platform_core::sentinel::ports::outbound::audit::audit_log_repository::AuditLogRepository;

pub struct PgAuditLogRepository {
    pool: PgPool,
}

impl PgAuditLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AuditLogRow {
    id: Uuid,
    guild_id: String,
    event_type: String,
    actor_id: Option<String>,
    actor_name: Option<String>,
    target_id: Option<String>,
    target_name: Option<String>,
    channel_id: Option<String>,
    channel_name: Option<String>,
    details: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<AuditLogRow> for AuditLog {
    fn from(row: AuditLogRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id.into(),
            event_type: row.event_type,
            actor_id: row.actor_id,
            actor_name: row.actor_name,
            target_id: row.target_id,
            target_name: row.target_name,
            channel_id: row.channel_id,
            channel_name: row.channel_name,
            details: row.details,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl AuditLogRepository for PgAuditLogRepository {
    async fn list_voice_channel_events(
        &self,
        channel_id: &str,
        limit: i64,
    ) -> Result<Vec<AuditLog>, DomainError> {
        let types: Vec<String> =
            platform_core::sentinel::domain::entities::audit::audit_log::VOICE_TIMELINE_EVENT_TYPES
                .iter()
                .map(|s| s.to_string())
                .collect();
        let rows: Vec<AuditLogRow> = sqlx::query_as(
            "SELECT id, guild_id, event_type, actor_id, actor_name, target_id, target_name, \
                    channel_id, channel_name, details, created_at \
             FROM audit_logs \
             WHERE channel_id = $1 AND event_type = ANY($2) \
             ORDER BY created_at ASC \
             LIMIT $3",
        )
        .bind(channel_id)
        .bind(&types)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("fetch voice events"))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn save(&self, log: &AuditLog) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO audit_logs (id, guild_id, event_type, actor_id, actor_name, target_id, target_name, channel_id, channel_name, details, created_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
        )
        .bind(log.id)
        .bind(log.guild_id.as_str())
        .bind(&log.event_type)
        .bind(&log.actor_id)
        .bind(&log.actor_name)
        .bind(log.target_id.as_deref())
        .bind(&log.target_name)
        .bind(log.channel_id.as_deref())
        .bind(&log.channel_name)
        .bind(&log.details)
        .bind(log.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn find_all(
        &self,
        guild_id: Option<&str>,
        filters: &AuditLogFilters,
    ) -> Result<Vec<AuditLog>, DomainError> {
        let (where_sql, next_idx) = build_where(guild_id, filters);
        let query = format!(
            "SELECT id, guild_id, event_type, actor_id, actor_name, target_id, target_name,                     channel_id, channel_name, details, created_at              FROM audit_logs {where_sql}              ORDER BY created_at DESC LIMIT ${next_idx} OFFSET ${}",
            next_idx + 1
        );
        let mut q = sqlx::query_as::<_, AuditLogRow>(&query);
        q = bind_where(q, guild_id, filters);
        q = q.bind(filters.limit).bind(filters.offset);
        let rows = q.fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows.into_iter().map(AuditLog::from).collect())
    }

    async fn count(
        &self,
        guild_id: Option<&str>,
        filters: &AuditLogFilters,
    ) -> Result<i64, DomainError> {
        let (where_sql, _) = build_where(guild_id, filters);
        let query = format!("SELECT COUNT(*) FROM audit_logs {where_sql}");
        let mut q = sqlx::query_scalar::<_, i64>(&query);
        q = bind_where_scalar(q, guild_id, filters);
        q.fetch_one(&self.pool).await.map_err(pg_err)
    }

    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM audit_logs WHERE guild_id = $1 AND created_at < NOW() - make_interval(days => $2)")
            .bind(guild_id)
            .bind(days)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("delete_audit_logs_older"))?;
        Ok(result.rows_affected())
    }
}

// ── Construction partagee de la clause WHERE ──
//
// `find_all` et `count` DOIVENT filtrer identiquement, sinon le total de
// pagination ne correspond pas aux lignes affichees. D'ou ces deux helpers
// plutot qu'une requete dupliquee.

/// Construit la clause WHERE et retourne l'index du prochain parametre libre.
fn build_where(guild_id: Option<&str>, filters: &AuditLogFilters) -> (String, u32) {
    let mut sql = String::from("WHERE 1=1");
    let mut idx = 1u32;
    if guild_id.is_some() {
        sql.push_str(&format!(" AND guild_id = ${idx}"));
        idx += 1;
    }
    if filters.event_type.is_some() {
        sql.push_str(&format!(" AND event_type = ${idx}"));
        idx += 1;
    }
    if !filters.event_types.is_empty() {
        sql.push_str(&format!(" AND event_type = ANY(${idx})"));
        idx += 1;
    }
    if filters.actor_id.is_some() {
        sql.push_str(&format!(" AND actor_id = ${idx}"));
        idx += 1;
    }
    if filters.target_id.is_some() {
        sql.push_str(&format!(" AND target_id = ${idx}"));
        idx += 1;
    }
    if filters.from.is_some() {
        sql.push_str(&format!(" AND created_at >= ${idx}"));
        idx += 1;
    }
    if filters.to.is_some() {
        sql.push_str(&format!(" AND created_at <= ${idx}"));
        idx += 1;
    }
    if filters.search.is_some() {
        // ILIKE sur les libelles seulement : `details` est du JSONB de forme
        // variable, l'indexer en texte couterait plus cher que le service rendu.
        sql.push_str(&format!(
            " AND (actor_name ILIKE ${idx} OR target_name ILIKE ${idx} OR channel_name ILIKE ${idx})"
        ));
        idx += 1;
    }
    (sql, idx)
}

macro_rules! bind_filters {
    ($q:expr, $guild_id:expr, $filters:expr) => {{
        let mut q = $q;
        if let Some(gid) = $guild_id {
            q = q.bind(gid.to_string());
        }
        if let Some(ref et) = $filters.event_type {
            q = q.bind(et.clone());
        }
        if !$filters.event_types.is_empty() {
            q = q.bind($filters.event_types.clone());
        }
        if let Some(ref aid) = $filters.actor_id {
            q = q.bind(aid.clone());
        }
        if let Some(ref tid) = $filters.target_id {
            q = q.bind(tid.clone());
        }
        if let Some(from) = $filters.from {
            q = q.bind(from);
        }
        if let Some(to) = $filters.to {
            q = q.bind(to);
        }
        if let Some(ref search) = $filters.search {
            q = q.bind(format!("%{search}%"));
        }
        q
    }};
}

fn bind_where<'a>(
    q: sqlx::query::QueryAs<'a, sqlx::Postgres, AuditLogRow, sqlx::postgres::PgArguments>,
    guild_id: Option<&str>,
    filters: &AuditLogFilters,
) -> sqlx::query::QueryAs<'a, sqlx::Postgres, AuditLogRow, sqlx::postgres::PgArguments> {
    bind_filters!(q, guild_id, filters)
}

fn bind_where_scalar<'a>(
    q: sqlx::query::QueryScalar<'a, sqlx::Postgres, i64, sqlx::postgres::PgArguments>,
    guild_id: Option<&str>,
    filters: &AuditLogFilters,
) -> sqlx::query::QueryScalar<'a, sqlx::Postgres, i64, sqlx::postgres::PgArguments> {
    bind_filters!(q, guild_id, filters)
}
