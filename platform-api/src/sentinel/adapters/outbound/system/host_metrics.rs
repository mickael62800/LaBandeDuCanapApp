//! Adapter sortant : métriques host (parsing du protocole `INFO` Redis,
//! lecture du snapshot host publie par ops-worker). Le handler HTTP
//! (`system/info.rs`) ne fait plus que l'assemblage et le mapping DTO.

use redis::AsyncCommands;
use serde::Deserialize;

const HOST_METRICS_KEY: &str = "ops:host-metrics";

/// Métriques Redis extraites de la sortie `INFO`.
#[derive(Debug, Default, Clone)]
pub struct RedisMetrics {
    pub used_memory_mb: u64,
    pub connected_clients: u64,
    pub total_keys: u64,
    pub uptime_seconds: u64,
}

/// Parse la sortie de `INFO` Redis (format "key:value" par ligne) et
/// extrait les champs qui nous interessent.
pub fn parse_redis_info(raw: &str) -> RedisMetrics {
    let mut m = RedisMetrics::default();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        match k {
            "used_memory" => {
                if let Ok(bytes) = v.parse::<u64>() {
                    m.used_memory_mb = bytes / 1024 / 1024;
                }
            }
            "connected_clients" => {
                m.connected_clients = v.parse().unwrap_or(0);
            }
            "uptime_in_seconds" => {
                m.uptime_seconds = v.parse().unwrap_or(0);
            }
            k if k.starts_with("db") => {
                // Ex: "db0:keys=1234,expires=56,avg_ttl=789"
                if let Some(keys_part) = v.split(',').find(|p| p.starts_with("keys=")) {
                    if let Some(n) = keys_part.strip_prefix("keys=") {
                        m.total_keys += n.parse::<u64>().unwrap_or(0);
                    }
                }
            }
            _ => {}
        }
    }
    m
}

/// Etat d'un disque / point de montage.
#[derive(Debug, Clone, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub fs_type: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub usage_percent: f32,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HostMetrics {
    pub cpu_percent: f32,
    pub cpu_cores: usize,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub disks: Vec<DiskInfo>,
}

/// Lit l'instantane ephemere publie par ops-worker. La cle Redis expire si
/// l'agent ne collecte plus, ce qui evite d'afficher indefiniment une mesure
/// obsolete comme si elle etait actuelle.
pub async fn load_host_metrics(conn: &mut redis::aio::MultiplexedConnection) -> HostMetrics {
    let payload: Option<String> = conn.get(HOST_METRICS_KEY).await.unwrap_or(None);
    payload
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_redis_info_handles_basic_fields() {
        let raw = "# Memory\nused_memory:1048576\nconnected_clients:42\nuptime_in_seconds:300\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.used_memory_mb, 1);
        assert_eq!(m.connected_clients, 42);
        assert_eq!(m.uptime_seconds, 300);
    }

    #[test]
    fn parse_redis_info_sums_keys_across_dbs() {
        let raw = "db0:keys=100,expires=10,avg_ttl=0\ndb1:keys=50,expires=5,avg_ttl=0\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.total_keys, 150);
    }

    #[test]
    fn parse_redis_info_ignores_comments_and_blank_lines() {
        let raw = "\n# Server\n\nused_memory:2097152\n# more comments\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.used_memory_mb, 2);
    }

    #[test]
    fn parse_redis_info_ignores_unknown_fields() {
        let raw = "some_other_field:xyz\nused_memory:1048576\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.used_memory_mb, 1);
        assert_eq!(m.connected_clients, 0);
    }

    #[test]
    fn parse_redis_info_handles_malformed_values_gracefully() {
        let raw = "connected_clients:not_a_number\nused_memory:also_bad\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.connected_clients, 0);
        assert_eq!(m.used_memory_mb, 0);
    }

    #[test]
    fn parse_redis_info_empty_input() {
        let m = parse_redis_info("");
        assert_eq!(m.used_memory_mb, 0);
        assert_eq!(m.connected_clients, 0);
        assert_eq!(m.uptime_seconds, 0);
        assert_eq!(m.total_keys, 0);
    }

    #[test]
    fn parse_redis_info_db_without_keys_prefix_ignored() {
        let raw = "db0:expires=10,avg_ttl=0\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.total_keys, 0);
    }

    #[test]
    fn parse_redis_info_used_memory_rounds_down() {
        // 1.5 Mo = 1 572 864 bytes → 1 Mo (division entiere)
        let raw = "used_memory:1572864\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.used_memory_mb, 1);
    }

    #[test]
    fn deserializes_ops_worker_host_snapshot() {
        let raw = r#"{
            "cpu_percent": 42.5,
            "cpu_cores": 8,
            "mem_used_mb": 4096,
            "mem_total_mb": 8192,
            "disks": [{
                "name": "/", "mount_point": "/", "fs_type": "host",
                "total_gb": 100.0, "used_gb": 25.0, "available_gb": 75.0,
                "usage_percent": 25.0, "is_removable": false
            }]
        }"#;
        let snapshot: HostMetrics = serde_json::from_str(raw).unwrap();
        assert_eq!(snapshot.cpu_cores, 8);
        assert_eq!(snapshot.mem_used_mb, 4096);
        assert_eq!(snapshot.disks.len(), 1);
        assert_eq!(snapshot.disks[0].mount_point, "/");
    }
}
