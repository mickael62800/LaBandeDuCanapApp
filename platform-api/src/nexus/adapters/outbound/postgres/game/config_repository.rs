use crate::nexus::adapters::outbound::postgres::pg_ctx;
use async_trait::async_trait;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::game_server_config_repository::GameServerConfigRepository;

pub struct PgGameServerConfigRepository {
    pool: PgPool,
}

impl PgGameServerConfigRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GameServerConfigRepository for PgGameServerConfigRepository {
    async fn get_all(&self, server_id: Uuid) -> Result<HashMap<String, String>, DomainError> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT config_key, config_value FROM game_server_configs WHERE server_id = $1",
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("get_all configs"))?;
        Ok(rows.into_iter().collect())
    }

    async fn upsert(
        &self,
        server_id: Uuid,
        key: &str,
        value: &str,
        updated_by: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO game_server_configs (server_id, config_key, config_value, updated_by) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (server_id, config_key) DO UPDATE SET \
                 config_value = EXCLUDED.config_value, \
                 updated_at = NOW(), \
                 updated_by = EXCLUDED.updated_by",
        )
        .bind(server_id)
        .bind(key)
        .bind(value)
        .bind(updated_by)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("upsert config"))?;
        Ok(())
    }

    async fn delete(&self, server_id: Uuid, key: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM game_server_configs WHERE server_id = $1 AND config_key = $2")
            .bind(server_id)
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("delete config"))?;
        Ok(())
    }

    async fn replace_all(
        &self,
        server_id: Uuid,
        entries: HashMap<String, String>,
        updated_by: Option<&str>,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_ctx("tx begin"))?;
        sqlx::query("DELETE FROM game_server_configs WHERE server_id = $1")
            .bind(server_id)
            .execute(&mut *tx)
            .await
            .map_err(pg_ctx("replace_all delete"))?;
        for (key, value) in &entries {
            sqlx::query(
                "INSERT INTO game_server_configs (server_id, config_key, config_value, updated_by) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(server_id)
            .bind(key)
            .bind(value)
            .bind(updated_by)
            .execute(&mut *tx)
            .await
            .map_err(pg_ctx("replace_all insert"))?;
        }
        tx.commit().await.map_err(pg_ctx("tx commit"))?;
        Ok(())
    }
}
