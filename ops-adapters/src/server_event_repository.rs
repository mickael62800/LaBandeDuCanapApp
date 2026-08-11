//! Adapter sortant Postgres de l'audit serveur (`server_events`). Tout le SQL
//! du domaine server_events vit ici.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::pg_err;
use ops_core::domain::entities::server_event::{NewServerEvent, ServerEvent, ServerEventFilter};
use ops_core::domain::errors::DomainError;

/// Taille max d'un lot d'insertion. Au-dela (recreation massive de conteneurs),
/// on decoupe pour ne pas construire une requete demesuree.
const BATCH_CHUNK: usize = 500;
use ops_core::ports::outbound::server_event_repository::ServerEventRepository;

pub struct PgServerEventRepository {
    pool: PgPool,
}

impl PgServerEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServerEventRepository for PgServerEventRepository {
    async fn record(
        &self,
        actor: &str,
        actor_name: Option<&str>,
        action: &str,
        target: Option<&str>,
        severity: &str,
        details: serde_json::Value,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO server_events (actor, actor_name, action, target, severity, details) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(actor)
        .bind(actor_name)
        .bind(action)
        .bind(target)
        .bind(severity)
        .bind(&details)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn record_batch(&self, events: &[NewServerEvent]) -> Result<(), DomainError> {
        if events.is_empty() {
            return Ok(());
        }
        for chunk in events.chunks(BATCH_CHUNK) {
            let mut builder = sqlx::QueryBuilder::new(
                "INSERT INTO server_events (actor, actor_name, action, target, severity, details) ",
            );
            builder.push_values(chunk, |mut row, event| {
                row.push_bind(&event.actor)
                    .push_bind(&event.actor_name)
                    .push_bind(&event.action)
                    .push_bind(&event.target)
                    .push_bind(&event.severity)
                    .push_bind(&event.details);
            });
            builder.build().execute(&self.pool).await.map_err(pg_err)?;
        }
        Ok(())
    }

    async fn list(&self, filter: &ServerEventFilter) -> Result<Vec<ServerEvent>, DomainError> {
        let mut sql = String::from(
            "SELECT id::text, timestamp, actor, actor_name, action, target, severity, details \
             FROM server_events WHERE 1=1",
        );
        let mut idx = 1;
        if filter.action_prefix.is_some() {
            sql.push_str(&format!(" AND action LIKE ${idx} || '%'"));
            idx += 1;
        }
        if filter.severity.is_some() {
            sql.push_str(&format!(" AND severity = ${idx}"));
            idx += 1;
        }
        sql.push_str(&format!(" ORDER BY timestamp DESC LIMIT ${idx}"));

        let mut q_builder = sqlx::query_as::<
            _,
            (
                String,
                chrono::DateTime<chrono::Utc>,
                Option<String>,
                Option<String>,
                String,
                Option<String>,
                String,
                serde_json::Value,
            ),
        >(&sql);
        if let Some(p) = &filter.action_prefix {
            q_builder = q_builder.bind(p);
        }
        if let Some(s) = &filter.severity {
            q_builder = q_builder.bind(s);
        }
        q_builder = q_builder.bind(filter.limit);

        let rows = q_builder.fetch_all(&self.pool).await.map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(
                |(id, timestamp, actor, actor_name, action, target, severity, details)| {
                    ServerEvent {
                        id,
                        timestamp,
                        actor,
                        actor_name,
                        action,
                        target,
                        severity,
                        details,
                    }
                },
            )
            .collect())
    }
}
