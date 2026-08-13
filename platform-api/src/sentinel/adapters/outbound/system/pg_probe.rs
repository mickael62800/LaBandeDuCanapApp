//! Adapter Postgres du port `SystemProbe` : sondes sante de la base.
//!
//! Seul endroit autorise a executer du SQL "sante" — les handlers inbound
//! (`system/info.rs`, `system/health.rs`) passent par le port.

use async_trait::async_trait;
use sqlx::PgPool;

use platform_core::ops::ports::outbound::system_probe::SystemProbe;
use platform_core::sentinel::domain::errors::DomainError;

pub struct PgSystemProbe {
    pool: PgPool,
}

impl PgSystemProbe {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SystemProbe for PgSystemProbe {
    async fn database_size_bytes(&self) -> Result<i64, DomainError> {
        sqlx::query_scalar("SELECT pg_database_size(current_database())")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))
    }

    async fn database_responding(&self) -> bool {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .is_ok()
    }
}
