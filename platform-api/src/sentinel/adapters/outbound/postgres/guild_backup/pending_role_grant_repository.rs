//! Adapter Postgres du port `PendingRoleGrantRepository`.
//!
//! Table `pending_role_grants` (PK (guild_id, user_id)), `role_ids` en JSONB.
//! `take` s'appuie sur `DELETE ... RETURNING` : la lecture ET la suppression
//! sont ATOMIQUES (idempotence du re-rolage au join). `upsert_many` fait un
//! upsert BATCH en une seule requete via `jsonb_to_recordset`.

use async_trait::async_trait;
use serde::Serialize;
use sqlx::types::Json;
use sqlx::PgPool;

use super::super::pg_ctx;
use platform_core::sentinel::domain::entities::guild_backup::pending_role_grant::PendingRoleGrant;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::guild_backup::pending_role_grant_repository::PendingRoleGrantRepository;

const TBL: &str = "pending_role_grants";

pub struct PgPendingRoleGrantRepository {
    pool: PgPool,
}

impl PgPendingRoleGrantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Ligne d'entree pour l'upsert batch (serialisee dans le tableau JSONB passe
/// a `jsonb_to_recordset`).
#[derive(Serialize)]
struct GrantRow<'a> {
    user_id: &'a str,
    role_ids: &'a [String],
}

#[async_trait]
impl PendingRoleGrantRepository for PgPendingRoleGrantRepository {
    async fn upsert_many(&self, grants: &[PendingRoleGrant]) -> Result<u64, DomainError> {
        if grants.is_empty() {
            return Ok(0);
        }
        // Tous les grants d'un meme appel partagent le meme guild_id (garanti
        // par le service). On l'extrait pour le lier une seule fois.
        let guild_id = &grants[0].guild_id;
        let rows: Vec<GrantRow> = grants
            .iter()
            .map(|g| GrantRow {
                user_id: &g.user_id,
                role_ids: &g.role_ids,
            })
            .collect();

        let res = sqlx::query(
            "INSERT INTO pending_role_grants (guild_id, user_id, role_ids) \
             SELECT $1, x.user_id, x.role_ids \
             FROM jsonb_to_recordset($2::jsonb) AS x(user_id TEXT, role_ids JSONB) \
             ON CONFLICT (guild_id, user_id) \
             DO UPDATE SET role_ids = EXCLUDED.role_ids, created_at = NOW()",
        )
        .bind(guild_id)
        .bind(Json(&rows))
        .execute(&self.pool)
        .await
        .map_err(pg_ctx(TBL))?;
        Ok(res.rows_affected())
    }

    async fn take(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Vec<String>>, DomainError> {
        // DELETE ... RETURNING : lecture + suppression atomiques.
        let row: Option<(Json<Vec<String>>,)> = sqlx::query_as(
            "DELETE FROM pending_role_grants WHERE guild_id = $1 AND user_id = $2 \
             RETURNING role_ids",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx(TBL))?;
        Ok(row.map(|(Json(r),)| r))
    }

    async fn clear_guild(&self, guild_id: &str) -> Result<u64, DomainError> {
        let res = sqlx::query("DELETE FROM pending_role_grants WHERE guild_id = $1")
            .bind(guild_id)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx(TBL))?;
        Ok(res.rows_affected())
    }
}
