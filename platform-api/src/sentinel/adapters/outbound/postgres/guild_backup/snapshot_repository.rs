//! Adapter Postgres du port `SnapshotRepository`.
//!
//! Stocke le `GuildSnapshot` en JSONB. `list` ne selectionne QUE les
//! metadonnees (pas le payload) pour la performance ; `get` charge le payload
//! complet (restauration).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::types::Json;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::pg_ctx;
use platform_core::sentinel::domain::entities::guild_backup::snapshot::GuildSnapshot;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::guild_backup::manage_snapshots::SnapshotSummary;
use platform_core::sentinel::ports::outbound::guild_backup::snapshot_repository::SnapshotRepository;

const TBL: &str = "guild_snapshots";

pub struct PgSnapshotRepository {
    pool: PgPool,
}

impl PgSnapshotRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Ligne de resume (sans le payload). Les compteurs roles/salons sont derives
/// cote SQL depuis le JSONB pour eviter de rapatrier tout le payload.
#[derive(sqlx::FromRow)]
struct SummaryRow {
    id: Uuid,
    guild_id: String,
    label: Option<String>,
    created_at: DateTime<Utc>,
    created_by: Option<String>,
    schema_version: i32,
    role_count: i64,
    channel_count: i64,
}

impl From<SummaryRow> for SnapshotSummary {
    fn from(r: SummaryRow) -> Self {
        SnapshotSummary {
            id: r.id,
            guild_id: r.guild_id,
            label: r.label.unwrap_or_default(),
            created_at: r.created_at.to_rfc3339(),
            created_by: r.created_by,
            schema_version: r.schema_version.max(0) as u32,
            role_count: r.role_count.max(0) as u32,
            channel_count: r.channel_count.max(0) as u32,
        }
    }
}

#[async_trait]
impl SnapshotRepository for PgSnapshotRepository {
    async fn insert(&self, snapshot: &GuildSnapshot) -> Result<Uuid, DomainError> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO guild_snapshots \
             (guild_id, label, schema_version, created_by, payload) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(&snapshot.guild_id)
        .bind(&snapshot.meta.label)
        .bind(snapshot.meta.schema_version as i32)
        .bind(&snapshot.meta.created_by)
        .bind(Json(snapshot))
        .fetch_one(&self.pool)
        .await
        .map_err(pg_ctx(TBL))?;
        Ok(id)
    }

    async fn list(&self, guild_id: &str) -> Result<Vec<SnapshotSummary>, DomainError> {
        let rows: Vec<SummaryRow> = sqlx::query_as(
            // jsonb_array_length renvoie un `integer` (int4) : on caste en
            // bigint (int8) pour matcher `role_count`/`channel_count: i64` cote
            // Rust — sinon sqlx echoue a decoder int4 dans un i64 des qu'il y a
            // au moins une ligne (500 "Erreur interne" sur la liste).
            "SELECT id, guild_id, label, created_at, created_by, schema_version, \
             COALESCE(jsonb_array_length(payload->'roles'), 0)::bigint AS role_count, \
             COALESCE(jsonb_array_length(payload->'channels'), 0)::bigint AS channel_count \
             FROM guild_snapshots WHERE guild_id = $1 ORDER BY created_at DESC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx(TBL))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get(&self, id: Uuid) -> Result<Option<GuildSnapshot>, DomainError> {
        let row: Option<(Json<GuildSnapshot>,)> =
            sqlx::query_as("SELECT payload FROM guild_snapshots WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_ctx(TBL))?;
        Ok(row.map(|(Json(s),)| s))
    }

    async fn delete(&self, id: Uuid) -> Result<bool, DomainError> {
        let res = sqlx::query("DELETE FROM guild_snapshots WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx(TBL))?;
        Ok(res.rows_affected() > 0)
    }

    async fn rename(&self, id: Uuid, label: &str) -> Result<bool, DomainError> {
        let res = sqlx::query("UPDATE guild_snapshots SET label = $2 WHERE id = $1")
            .bind(id)
            .bind(label)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx(TBL))?;
        Ok(res.rows_affected() > 0)
    }

    async fn count(&self, guild_id: &str) -> Result<u32, DomainError> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM guild_snapshots WHERE guild_id = $1")
            .bind(guild_id)
            .fetch_one(&self.pool)
            .await
            .map_err(pg_ctx(TBL))?;
        Ok(n.max(0) as u32)
    }

    async fn oldest_id(&self, guild_id: &str) -> Result<Option<Uuid>, DomainError> {
        let id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM guild_snapshots WHERE guild_id = $1 \
             ORDER BY created_at ASC LIMIT 1",
        )
        .bind(guild_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx(TBL))?;
        Ok(id)
    }
}
