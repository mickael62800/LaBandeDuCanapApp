use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::super::pg_ctx;
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::backup_repository::GameBackupRepository;

pub struct PgGameBackupRepository {
    pool: PgPool,
}

impl PgGameBackupRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GameBackupRepository for PgGameBackupRepository {
    async fn last_auto_backup_at(
        &self,
        server_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        // `backup_type = 'auto'` : une archive declenchee a la main ne doit pas
        // repousser la sauvegarde automatique du lendemain.
        sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
            "SELECT MAX(created_at) FROM game_backups \
             WHERE server_id = $1 AND backup_type = 'auto'",
        )
        .bind(server_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_ctx("last auto backup"))
    }

    async fn record(
        &self,
        server_id: Uuid,
        file_path: &str,
        size_bytes: i64,
        backup_type: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO game_backups (server_id, file_path, size_bytes, backup_type) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(server_id)
        .bind(file_path)
        .bind(size_bytes)
        .bind(backup_type)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(pg_ctx("record backup"))
    }
}
