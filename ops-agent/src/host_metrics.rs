//! Collecte les ressources de la machine hote et publie un snapshot dans Redis.
//!
//! Le worker lit `/host/proc`, monte en lecture seule par Compose. Sentinel API
//! ne partage ainsi plus le namespace PID de l'hote et ne collecte plus ces
//! informations elle-meme.

use std::time::Duration;

use redis::AsyncCommands;
use serde::Serialize;

pub const REDIS_KEY: &str = "ops:host-metrics";

#[derive(Debug, Serialize)]
struct HostMetricsSnapshot {
    cpu_percent: f32,
    cpu_cores: usize,
    mem_used_mb: u64,
    mem_total_mb: u64,
    disks: Vec<HostDisk>,
}

#[derive(Debug, Serialize)]
struct HostDisk {
    name: String,
    mount_point: String,
    fs_type: String,
    total_gb: f64,
    used_gb: f64,
    available_gb: f64,
    usage_percent: f32,
    is_removable: bool,
}

#[derive(Debug, Clone, Copy)]
struct CpuSample {
    idle: u64,
    total: u64,
    cores: usize,
}

pub fn spawn(redis_client: redis::Client) {
    let interval_secs = std::env::var("HOST_METRICS_INTERVAL_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(30);

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let snapshot = match tokio::task::spawn_blocking(collect).await {
                Ok(Ok(snapshot)) => snapshot,
                Ok(Err(error)) => {
                    tracing::warn!(%error, "host metrics: collecte impossible");
                    continue;
                }
                Err(error) => {
                    tracing::warn!(%error, "host metrics: tache de collecte interrompue");
                    continue;
                }
            };
            let Ok(payload) = serde_json::to_string(&snapshot) else {
                continue;
            };
            let Ok(mut conn) = redis_client.get_multiplexed_async_connection().await else {
                tracing::warn!("host metrics: Redis indisponible");
                continue;
            };
            let result: redis::RedisResult<()> =
                conn.set_ex(REDIS_KEY, payload, interval_secs * 4).await;
            if let Err(error) = result {
                tracing::warn!(%error, "host metrics: publication Redis impossible");
            }
        }
    });
}

fn collect() -> Result<HostMetricsSnapshot, String> {
    let first = read_cpu("/host/proc/stat")?;
    std::thread::sleep(Duration::from_millis(200));
    let second = read_cpu("/host/proc/stat")?;
    let total_delta = second.total.saturating_sub(first.total);
    let idle_delta = second.idle.saturating_sub(first.idle);
    let cpu_percent = if total_delta == 0 {
        0.0
    } else {
        100.0 * total_delta.saturating_sub(idle_delta) as f32 / total_delta as f32
    };
    let (mem_used_mb, mem_total_mb) = read_memory("/host/proc/meminfo")?;

    Ok(HostMetricsSnapshot {
        cpu_percent,
        cpu_cores: second.cores,
        mem_used_mb,
        mem_total_mb,
        disks: read_disks("/var/lib/sentinel/disks-current.json"),
    })
}

fn read_cpu(path: &str) -> Result<CpuSample, String> {
    let raw = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    parse_cpu(&raw).ok_or_else(|| "format /proc/stat invalide".to_string())
}

fn parse_cpu(raw: &str) -> Option<CpuSample> {
    let mut lines = raw.lines();
    let aggregate = lines.next()?.split_whitespace().collect::<Vec<_>>();
    if aggregate.first().copied() != Some("cpu") {
        return None;
    }
    let values = aggregate[1..]
        .iter()
        .map(|value| value.parse::<u64>())
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let total = values.iter().sum();
    let idle = values.get(3).copied().unwrap_or(0) + values.get(4).copied().unwrap_or(0);
    let cores = raw
        .lines()
        .filter(|line| {
            line.split_whitespace().next().is_some_and(|name| {
                name.strip_prefix("cpu")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
        })
        .count();
    Some(CpuSample { idle, total, cores })
}

fn read_memory(path: &str) -> Result<(u64, u64), String> {
    let raw = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    parse_memory(&raw).ok_or_else(|| "format /proc/meminfo invalide".to_string())
}

fn parse_memory(raw: &str) -> Option<(u64, u64)> {
    let value = |name: &str| {
        raw.lines().find_map(|line| {
            let (key, rest) = line.split_once(':')?;
            (key == name).then(|| rest.split_whitespace().next()?.parse::<u64>().ok())?
        })
    };
    let total_kb = value("MemTotal")?;
    let available_kb = value("MemAvailable")?;
    Some((
        (total_kb.saturating_sub(available_kb)) / 1024,
        total_kb / 1024,
    ))
}

fn read_disks(path: &str) -> Vec<HostDisk> {
    #[derive(serde::Deserialize)]
    struct Snapshot {
        disks: Vec<Disk>,
    }
    #[derive(serde::Deserialize)]
    struct Disk {
        mount: String,
        #[serde(default)]
        used_gb: f64,
        #[serde(default)]
        total_gb: f64,
        #[serde(default)]
        usage_pct: f32,
    }

    let Some(snapshot) = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Snapshot>(&raw).ok())
    else {
        return Vec::new();
    };
    snapshot
        .disks
        .into_iter()
        .map(|disk| HostDisk {
            name: disk.mount.clone(),
            mount_point: disk.mount,
            fs_type: "host".into(),
            total_gb: disk.total_gb,
            used_gb: disk.used_gb,
            available_gb: (disk.total_gb - disk.used_gb).max(0.0),
            usage_percent: disk.usage_pct,
            is_removable: false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_cpu_and_core_count() {
        let sample = parse_cpu("cpu  10 2 3 40 5 0 0 0 0 0\ncpu0 1 0 0 1\ncpu1 1 0 0 1\n").unwrap();
        assert_eq!(sample.total, 60);
        assert_eq!(sample.idle, 45);
        assert_eq!(sample.cores, 2);
    }

    #[test]
    fn parses_available_host_memory() {
        assert_eq!(
            parse_memory("MemTotal: 8192 kB\nMemAvailable: 3072 kB\n"),
            Some((5, 8))
        );
    }
}
