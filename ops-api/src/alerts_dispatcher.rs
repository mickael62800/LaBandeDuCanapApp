//! Worker periodique de supervision : evalue des regles d'alerte CONFIGURABLES
//! (table `alert_rules`) contre les indicateurs collectes (ressources host,
//! services offline, auth failures, cert TLS, conteneurs) et envoie un webhook
//! Discord quand un seuil est franchi.
//!
//! Config via env :
//!   SECURITY_ALERTS_WEBHOOK        (URL webhook Discord — requis, sinon desactive)
//!   SECURITY_ALERTS_INTERVAL_SECS  (defaut 300 = 5 min)
//!
//! Les SEUILS sont dans la table `alert_rules` (voir migration 358), editables
//! sans redeploiement. La deduplication/cooldown est persistee dans Redis
//! (cle `alert:sent:*` a TTL = cooldown de la regle) → survit au redemarrage.

use std::sync::Arc;
use std::time::Duration;

use redis::AsyncCommands;
use sqlx::PgPool;
use tokio::sync::RwLock;

use ops_core::domain::entities::container_monitor::{ContainerChangeKind, ContainerMonitorState};

/// Une regle d'alerte chargee depuis la DB.
#[derive(Debug, Clone, sqlx::FromRow)]
struct AlertRule {
    id: String,
    label: String,
    metric: String,
    comparator: String,
    threshold: Option<f64>,
    severity: String,
    cooldown_secs: i32,
}

impl AlertRule {
    /// `true` si la valeur numerique observee franchit le seuil.
    fn triggers(&self, value: f64) -> bool {
        match (self.comparator.as_str(), self.threshold) {
            ("gt", Some(t)) => value > t,
            ("lt", Some(t)) => value < t,
            _ => false,
        }
    }

    fn color(&self) -> u32 {
        match self.severity.as_str() {
            "critical" => 0xE74C3C,
            "info" => 0x3498DB,
            _ => 0xF39C12, // warning
        }
    }
}

/// Instantane des indicateurs, calcule une fois par tick.
struct Metrics {
    cpu_percent: f64,
    mem_percent: f64,
    disk_percent: f64,
    auth_failures_1h: f64,
    tls_expiry_days: Option<f64>,
    offline_services: Vec<String>,
    container_changes: Vec<(String, String, String)>, // (name, kind, ts)
}

pub fn spawn(
    pg_pool: PgPool,
    redis_client: redis::Client,
    container_state: Option<Arc<RwLock<ContainerMonitorState>>>,
) {
    let webhook = match std::env::var("SECURITY_ALERTS_WEBHOOK") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            tracing::info!("SECURITY_ALERTS_WEBHOOK non defini, alertes desactivees");
            return;
        }
    };
    let interval_secs: u64 = std::env::var("SECURITY_ALERTS_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);

    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("alerts client: {e}");
                return;
            }
        };

        loop {
            tokio::time::sleep(Duration::from_secs(interval_secs)).await;

            let rules = match load_rules(&pg_pool).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(error = %e, "alerts: chargement des regles echoue");
                    continue;
                }
            };
            if rules.is_empty() {
                continue;
            }

            let metrics = collect_metrics(&pg_pool, &redis_client, &container_state).await;

            for rule in &rules {
                for (key_suffix, content) in evaluate(rule, &metrics) {
                    let key = format!("alert:sent:{}:{}", rule.id, key_suffix);
                    if !claim_dedup(&redis_client, &key, rule.cooldown_secs).await {
                        continue; // deja envoye dans la fenetre de cooldown
                    }
                    send_webhook(&client, &webhook, &content, rule.color()).await;
                }
            }
        }
    });
}

/// Charge les regles actives.
async fn load_rules(pg_pool: &PgPool) -> Result<Vec<AlertRule>, sqlx::Error> {
    sqlx::query_as::<_, AlertRule>(
        "SELECT id, label, metric, comparator, threshold, severity, cooldown_secs \
         FROM alert_rules WHERE enabled = TRUE",
    )
    .fetch_all(pg_pool)
    .await
}

/// Evalue une regle contre l'instantane. Renvoie 0..N alertes a emettre, chacune
/// avec un suffixe de cle de dedup (ex. par service offline) + le contenu.
fn evaluate(rule: &AlertRule, m: &Metrics) -> Vec<(String, String)> {
    match rule.metric.as_str() {
        "cpu_percent" if rule.triggers(m.cpu_percent) => vec![(
            "_".into(),
            format!("🔥 **{}** : CPU host à {:.0}%", rule.label, m.cpu_percent),
        )],
        "mem_percent" if rule.triggers(m.mem_percent) => vec![(
            "_".into(),
            format!("🧠 **{}** : RAM host à {:.0}%", rule.label, m.mem_percent),
        )],
        "disk_percent" if rule.triggers(m.disk_percent) => vec![(
            "_".into(),
            format!("💾 **{}** : disque à {:.0}%", rule.label, m.disk_percent),
        )],
        "auth_failures_1h" if rule.triggers(m.auth_failures_1h) => vec![(
            // Cle par heure : ré-alerte au plus une fois par heure meme si le
            // cooldown est plus court.
            chrono::Utc::now().format("%Y%m%d%H").to_string(),
            format!(
                "🚨 **{}** : {:.0} échecs d'auth sur 1h",
                rule.label, m.auth_failures_1h
            ),
        )],
        "tls_expiry_days" => match m.tls_expiry_days {
            Some(days) if rule.triggers(days) => vec![(
                format!("{days:.0}"),
                format!(
                    "🔐 **{}** : cert TLS expire dans {:.0} jours",
                    rule.label, days
                ),
            )],
            _ => vec![],
        },
        "service_offline" => m
            .offline_services
            .iter()
            .map(|name| {
                (
                    name.clone(),
                    format!("📴 **{}** : `{}` ne répond plus", rule.label, name),
                )
            })
            .collect(),
        "container_removed" => m
            .container_changes
            .iter()
            .map(|(name, kind, ts)| {
                (
                    format!("{name}-{ts}"),
                    format!("🐳 **{}** : `{}` ({})", rule.label, name, kind),
                )
            })
            .collect(),
        _ => vec![],
    }
}

/// Calcule l'instantane des indicateurs.
async fn collect_metrics(
    pg_pool: &PgPool,
    redis_client: &redis::Client,
    container_state: &Option<Arc<RwLock<ContainerMonitorState>>>,
) -> Metrics {
    // ── Ressources host (sysinfo, necessite pid: host cote compose) ──
    let (cpu_percent, mem_percent, disk_percent) = collect_host_resources();

    // ── Auth failures (1h) ──
    let auth_failures_1h = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM logs WHERE created_at > NOW() - INTERVAL '1 hour' \
         AND status_code IN (401, 403)",
    )
    .fetch_one(pg_pool)
    .await
    .unwrap_or(0) as f64;

    // ── TLS expiry (shim host) ──
    let tls_expiry_days = std::fs::read_to_string("/var/lib/sentinel/tls-cert.json")
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("days_until_expiry").and_then(|x| x.as_i64()))
        .map(|d| d as f64);

    // ── Services offline (Redis : bots:known + bot:online:{name}) ──
    let offline_services = collect_offline_services(redis_client).await;

    // ── Conteneurs modifies (monitor en memoire) ──
    let mut container_changes = Vec::new();
    if let Some(cs) = container_state {
        let s = cs.read().await;
        for c in &s.recent_changes {
            if matches!(c.kind, ContainerChangeKind::Removed | ContainerChangeKind::ImageChanged) {
                container_changes.push((
                    c.container.name.clone(),
                    c.kind.as_action().to_owned(),
                    c.timestamp.clone(),
                ));
            }
        }
    }

    Metrics {
        cpu_percent,
        mem_percent,
        disk_percent,
        auth_failures_1h,
        tls_expiry_days,
        offline_services,
        container_changes,
    }
}

/// CPU %, RAM %, pire disque % (via sysinfo). Renvoie (0,0,0) si indisponible.
fn collect_host_resources() -> (f64, f64, f64) {
    use sysinfo::{Disks, System};

    let mut sys = System::new();
    // CPU : deux echantillons espaces pour une mesure fiable (cf. info.rs).
    sys.refresh_cpu_usage();
    std::thread::sleep(Duration::from_millis(200));
    sys.refresh_cpu_usage();
    let cpu = sys.global_cpu_usage() as f64;

    sys.refresh_memory();
    let total = sys.total_memory();
    let mem = if total > 0 {
        (sys.used_memory() as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    // Pire taux d'occupation parmi les disques reels (> 1 Gio, ignore tmpfs).
    let disks = Disks::new_with_refreshed_list();
    let disk = disks
        .list()
        .iter()
        .filter(|d| d.total_space() > 1024 * 1024 * 1024)
        .map(|d| {
            let t = d.total_space();
            let used = t.saturating_sub(d.available_space());
            if t > 0 {
                (used as f64 / t as f64) * 100.0
            } else {
                0.0
            }
        })
        .fold(0.0_f64, f64::max);

    (cpu, mem, disk)
}

/// Liste des services connus (`bots:known`) sans heartbeat (`bot:online:{name}`).
async fn collect_offline_services(redis_client: &redis::Client) -> Vec<String> {
    let Ok(mut conn) = redis_client.get_multiplexed_async_connection().await else {
        return Vec::new();
    };
    let known: Vec<String> = conn.smembers("bots:known").await.unwrap_or_default();
    let mut offline = Vec::new();
    for name in known {
        let online: bool = conn
            .exists(format!("bot:online:{name}"))
            .await
            .unwrap_or(true); // en cas d'erreur Redis, on ne crie pas au loup
        if !online {
            offline.push(name);
        }
    }
    offline
}

/// Reserve atomiquement l'envoi via une cle Redis `SET NX EX cooldown`.
/// Renvoie `true` si la cle a ete posee (= premier envoi dans la fenetre),
/// `false` si elle existait deja (cooldown en cours). Fail-open si Redis KO
/// (on prefere une eventuelle alerte doublon a une alerte manquee).
async fn claim_dedup(redis_client: &redis::Client, key: &str, cooldown_secs: i32) -> bool {
    let Ok(mut conn) = redis_client.get_multiplexed_async_connection().await else {
        return true;
    };
    let ttl = cooldown_secs.max(1) as usize;
    let res: redis::RedisResult<Option<String>> = redis::cmd("SET")
        .arg(key)
        .arg("1")
        .arg("NX")
        .arg("EX")
        .arg(ttl)
        .query_async(&mut conn)
        .await;
    matches!(res, Ok(Some(_)))
}

/// Envoie une alerte sur le webhook Discord.
async fn send_webhook(client: &reqwest::Client, webhook: &str, content: &str, color: u32) {
    let body = serde_json::json!({
        "username": "DiscordSentinel · Supervision",
        "embeds": [{
            "title": "Alerte serveur",
            "description": content,
            "color": color,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }]
    });
    match client.post(webhook).json(&body).send().await {
        Ok(r) if r.status().is_success() => {}
        Ok(r) => tracing::warn!(status = %r.status(), "alerte webhook : status non-2xx"),
        Err(e) => tracing::warn!(?e, "alerte webhook : erreur envoi"),
    }
}

#[cfg(test)]
#[path = "tests/alerts_dispatcher.rs"]
mod tests;
