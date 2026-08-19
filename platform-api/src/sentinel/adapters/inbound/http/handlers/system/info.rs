//! GET /api/system/info — etat detaille du systeme pour le panneau d'admin web.
//!
//! Retourne :
//!   - la liste nominative des bots/workers connus avec leur etat online,
//!   - les metriques CPU/RAM de l'host collectees par ops-worker,
//!   - les metriques CPU/RAM du process API lui-meme,
//!   - l'uptime du process API,
//!   - la taille de la base de donnees PostgreSQL.
//!
//! Sources :
//!   - `bots:known` (Redis SET) + `bot:online:{name}` (EXISTS avec TTL 90s)
//!     pour la liste + etat des services.
//!   - `ops:host-metrics` (Redis) pour les ressources de l'hote.
//!   - `sysinfo` uniquement pour le processus API.
//!   - `STARTED_AT` (OnceLock initialise au demarrage) pour l'uptime.
//!   - `pg_database_size(current_database())` pour la taille BDD.

use std::sync::OnceLock;
use std::time::Instant;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::adapters::outbound::system::host_metrics::{
    load_host_metrics, parse_redis_info, DiskInfo, HostMetrics, InternetProbe, RedisMetrics,
};
use crate::sentinel::bootstrap::state::OpsState;
use axum::extract::{Extension, State};
use axum::Json;
use redis::AsyncCommands;
use serde::Serialize;
use sysinfo::ProcessRefreshKind;
use sysinfo::RefreshKind;
use sysinfo::System;

/// Moment de demarrage du process API. Initialise une seule fois au premier
/// appel (ou explicitement depuis main.rs via `record_startup()`).
static STARTED_AT: OnceLock<Instant> = OnceLock::new();

/// A appeler depuis main.rs pour fixer l'uptime reel. Si non appele, le
/// premier appel a l'endpoint fixera la valeur.
pub fn record_startup() {
    let _ = STARTED_AT.set(Instant::now());
}

fn uptime_seconds() -> u64 {
    STARTED_AT.get_or_init(Instant::now).elapsed().as_secs()
}

#[derive(Debug, Serialize)]
pub struct ServiceStatusDto {
    pub name: String,
    pub online: bool,
}

#[derive(Debug, Serialize)]
pub struct HostMetricsDto {
    pub cpu_percent: f32,
    /// Processeurs LOGIQUES (threads), pas coeurs physiques : c'est ce que le
    /// noyau expose, et la bonne reference pour comparer la charge.
    pub cpu_cores: usize,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    /// Debit reseau instantane de l'hote, en octets par seconde. Un debit et
    /// non un compteur : les octets cumules depuis le demarrage ne disent rien
    /// a qui les lit.
    pub net_rx_bytes_per_sec: u64,
    pub net_tx_bytes_per_sec: u64,
    /// Joignabilite des services dont la plateforme depend (Discord d'abord).
    pub internet: Vec<InternetProbe>,
    /// Charge moyenne (1 et 5 min), a comparer au nombre de coeurs : une
    /// charge superieure aux coeurs signifie que des taches attendent.
    pub load_1m: f32,
    pub load_5m: f32,
}

#[derive(Debug, Serialize)]
pub struct ProcessMetricsDto {
    pub cpu_percent: f32,
    pub mem_used_mb: u64,
}

#[derive(Debug, Serialize, Default)]
pub struct RedisMetricsDto {
    pub used_memory_mb: u64,
    pub connected_clients: u64,
    pub total_keys: u64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct DiskDto {
    pub name: String,
    pub mount_point: String,
    pub fs_type: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub usage_percent: f32,
    pub is_removable: bool,
}

#[derive(Debug, Serialize)]
pub struct HealthChecksDto {
    pub api_responding: bool,
    pub postgres_responding: bool,
    pub redis_responding: bool,
}

#[derive(Debug, Serialize)]
pub struct SystemInfoDto {
    pub bots: Vec<ServiceStatusDto>,
    pub workers: Vec<ServiceStatusDto>,
    pub host: HostMetricsDto,
    pub process: ProcessMetricsDto,
    pub redis: RedisMetricsDto,
    pub disks: Vec<DiskDto>,
    pub health: HealthChecksDto,
    pub uptime_seconds: u64,
    pub db_size_mb: u64,
}

impl From<RedisMetrics> for RedisMetricsDto {
    fn from(m: RedisMetrics) -> Self {
        Self {
            used_memory_mb: m.used_memory_mb,
            connected_clients: m.connected_clients,
            total_keys: m.total_keys,
            uptime_seconds: m.uptime_seconds,
        }
    }
}

impl From<DiskInfo> for DiskDto {
    fn from(d: DiskInfo) -> Self {
        Self {
            name: d.name,
            mount_point: d.mount_point,
            fs_type: d.fs_type,
            total_gb: d.total_gb,
            used_gb: d.used_gb,
            available_gb: d.available_gb,
            usage_percent: d.usage_percent,
            is_removable: d.is_removable,
        }
    }
}

pub async fn get_system_info(
    State(state): State<OpsState>,
    _user: Option<Extension<WebUser>>,
) -> Result<Json<SystemInfoDto>, ApiError> {
    // SECURITE : cet endpoint divulgue des infos host (CPU/RAM, points de
    // montage disques, taille BDD, liste des services). Le gate user global ne
    // filtre que les mutations (GET = pass) : sans ce check, tout porteur d'un
    // X-Discord-Token valide (meme viewer) y accederait. On restreint donc aux
    // superadmins, comme les endpoints d'admin host (docker).
    // Appel web -> WebUser present -> exige superadmin. Appel interne
    // (bot/worker, AuthKind::Internal, pas de WebUser) -> autorise.

    // ── 1. Liste nominative + metriques Redis ──
    let (mut bots, mut workers) = (Vec::new(), Vec::new());
    let mut redis_metrics = RedisMetricsDto::default();
    let mut host_metrics = HostMetrics::default();
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        let known: Vec<String> = conn.smembers("bots:known").await.unwrap_or_default();
        for name in known {
            let online: bool = conn
                .exists::<_, bool>(format!("bot:online:{}", name))
                .await
                .unwrap_or(false);
            let entry = ServiceStatusDto {
                name: name.clone(),
                online,
            };
            if platform_core::sentinel::domain::entities::system::config_parsers::is_worker_service(
                &name,
            ) {
                workers.push(entry);
            } else {
                bots.push(entry);
            }
        }

        // INFO Redis — memoire, clients, uptime, nb de cles
        let raw: String = redis::cmd("INFO")
            .query_async(&mut conn)
            .await
            .unwrap_or_default();
        redis_metrics = parse_redis_info(&raw).into();
        host_metrics = load_host_metrics(&mut conn).await;
    }
    bots.sort_by(|a, b| a.name.cmp(&b.name));
    workers.sort_by(|a, b| a.name.cmp(&b.name));

    // ── 2. Metriques du processus API via sysinfo ──
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
    );
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    // Process API : on utilise le PID courant.
    let (proc_cpu, proc_mem_mb) = {
        let pid = sysinfo::get_current_pid().ok();
        match pid.and_then(|p| sys.process(p)) {
            Some(p) => (p.cpu_usage(), p.memory() / 1024 / 1024),
            None => (0.0, 0),
        }
    };

    // ── 3. Taille BDD PostgreSQL + health check (via le port SystemProbe) ──
    let db_size_bytes: i64 = state.system_probe.database_size_bytes().await.unwrap_or(-1);
    let postgres_responding = db_size_bytes >= 0;
    let db_size_mb = if db_size_bytes > 0 {
        (db_size_bytes / 1024 / 1024) as u64
    } else {
        0
    };

    // ── 4. Health check Redis (PING) ──
    let redis_responding =
        if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
            redis::cmd("PING")
                .query_async::<String>(&mut conn)
                .await
                .is_ok()
        } else {
            false
        };

    // ── 5. Disques collectes par ops-worker dans le meme snapshot ──
    let disks: Vec<DiskDto> = host_metrics.disks.into_iter().map(Into::into).collect();

    Ok(Json(SystemInfoDto {
        bots,
        workers,
        host: HostMetricsDto {
            internet: host_metrics.internet.clone(),
            load_1m: host_metrics.load_1m,
            load_5m: host_metrics.load_5m,
            net_rx_bytes_per_sec: host_metrics.net_rx_bytes_per_sec,
            net_tx_bytes_per_sec: host_metrics.net_tx_bytes_per_sec,
            cpu_percent: host_metrics.cpu_percent,
            cpu_cores: host_metrics.cpu_cores,
            mem_used_mb: host_metrics.mem_used_mb,
            mem_total_mb: host_metrics.mem_total_mb,
        },
        process: ProcessMetricsDto {
            cpu_percent: proc_cpu,
            mem_used_mb: proc_mem_mb,
        },
        redis: redis_metrics,
        disks,
        health: HealthChecksDto {
            api_responding: true, // si on est ici, l'API repond
            postgres_responding,
            redis_responding,
        },
        uptime_seconds: uptime_seconds(),
        db_size_mb,
    }))
}

#[cfg(test)]
#[path = "tests/info.rs"]
mod tests;
