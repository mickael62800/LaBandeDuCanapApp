use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::pg_ctx;
use platform_core::nexus::domain::entities::game::alert::{AlertKind, AlertSettings};
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::alert_repository::{
    GameAlertRepository, ServerAlertConfig,
};

pub struct PgGameAlertRepository {
    pool: PgPool,
}

impl PgGameAlertRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type AlertRow = (
    Uuid,
    String,
    i32,
    i32,
    i32,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
);

const COLS: &str = "server_id, webhook_url, cpu_threshold, ram_threshold, latency_threshold_ms, \
     last_cpu_alert_at, last_ram_alert_at, last_latency_alert_at";

fn to_config(row: AlertRow) -> ServerAlertConfig {
    ServerAlertConfig {
        server_id: row.0,
        webhook_url: row.1,
        settings: AlertSettings {
            cpu_threshold: row.2,
            ram_threshold: row.3,
            latency_threshold_ms: row.4,
            last_cpu_alert_at: row.5,
            last_ram_alert_at: row.6,
            last_latency_alert_at: row.7,
        },
    }
}

#[async_trait]
impl GameAlertRepository for PgGameAlertRepository {
    async fn find(&self, server_id: Uuid) -> Result<Option<ServerAlertConfig>, DomainError> {
        let row: Option<AlertRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM game_server_alerts WHERE server_id = $1"
        ))
        .bind(server_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("find alert config"))?;
        Ok(row.map(to_config))
    }

    async fn upsert(
        &self,
        server_id: Uuid,
        webhook_url: &str,
        cpu_threshold: i32,
        ram_threshold: i32,
        latency_threshold_ms: i32,
        actor: Option<&str>,
    ) -> Result<(), DomainError> {
        // Les dates de dernier envoi ne sont PAS touchees : changer un seuil ne
        // doit pas rouvrir la porte a une salve d'alertes deja envoyees.
        sqlx::query(
            "INSERT INTO game_server_alerts \
                 (server_id, webhook_url, cpu_threshold, ram_threshold, latency_threshold_ms, updated_by) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (server_id) DO UPDATE SET \
                 webhook_url = EXCLUDED.webhook_url, \
                 cpu_threshold = EXCLUDED.cpu_threshold, \
                 ram_threshold = EXCLUDED.ram_threshold, \
                 latency_threshold_ms = EXCLUDED.latency_threshold_ms, \
                 updated_at = NOW(), \
                 updated_by = EXCLUDED.updated_by",
        )
        .bind(server_id)
        .bind(webhook_url)
        .bind(cpu_threshold)
        .bind(ram_threshold)
        .bind(latency_threshold_ms)
        .bind(actor)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("upsert alert config"))?;
        Ok(())
    }

    async fn delete(&self, server_id: Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query("DELETE FROM game_server_alerts WHERE server_id = $1")
            .bind(server_id)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("delete alert config"))?;
        Ok(result.rows_affected() > 0)
    }

    async fn mark_sent(&self, server_id: Uuid, kind: AlertKind) -> Result<(), DomainError> {
        // La colonne depend de la nature, mais elle vient d'une enumeration du
        // domaine : aucune valeur exterieure n'atteint cette requete.
        let sql = match kind {
            AlertKind::Cpu => {
                "UPDATE game_server_alerts SET last_cpu_alert_at = NOW() WHERE server_id = $1"
            }
            AlertKind::Ram => {
                "UPDATE game_server_alerts SET last_ram_alert_at = NOW() WHERE server_id = $1"
            }
            AlertKind::Latency => {
                "UPDATE game_server_alerts SET last_latency_alert_at = NOW() WHERE server_id = $1"
            }
        };
        sqlx::query(sql)
            .bind(server_id)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("mark alert sent"))?;
        Ok(())
    }
}
