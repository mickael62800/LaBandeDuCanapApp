//! Evalue periodiquement les regles d'alerte configurables.
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

use redis::aio::MultiplexedConnection;
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
    /// `None` = comptage indisponible (erreur SQL) : on ne doit surtout pas le
    /// confondre avec un zero reel, sinon une requete cassee ferait passer une
    /// panne pour « aucun echec » et l'alerte ne se declencherait jamais.
    auth_failures_1h: Option<f64>,
    tls_expiry_days: Option<f64>,
    offline_services: Vec<String>,
    container_changes: Vec<(String, String, String)>, // (name, kind, ts)
}

/// Une alerte prete a etre reservee puis envoyee.
struct Candidate {
    key: String,
    content: String,
    color: u32,
    cooldown_secs: i32,
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

        // `interval` plutot que `sleep` en tete de boucle : la premiere
        // evaluation a lieu tout de suite (pas apres un intervalle complet), et
        // la cadence ne derive pas de la duree de chaque traitement. `Skip` :
        // si un cycle (SQL, Docker, webhook) deborde, on ne rattrape pas les
        // ticks manques en rafale, on reprend au tick suivant.
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;

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

            // Une seule connexion multiplexee par tick, partagee par toutes les
            // operations Redis du cycle (curseur, services offline, dedup). Les
            // clones partagent le meme pipe : plus d'ouverture de connexion par
            // sous-appel. Recreee a chaque tour, donc resiliente a une coupure
            // Redis. `None` = Redis indisponible ce cycle : fail-open partout.
            let redis_conn = redis_client.get_multiplexed_async_connection().await.ok();
            if redis_conn.is_none() {
                tracing::warn!(
                    "alerts: connexion Redis indisponible ce cycle (dedup et services degrades)"
                );
            }

            let metrics = collect_metrics(&pg_pool, redis_conn.clone(), &container_state).await;

            // 1. Construire toutes les alertes candidates du cycle.
            let mut candidates: Vec<Candidate> = Vec::new();
            for rule in &rules {
                for (key_suffix, content) in evaluate(rule, &metrics) {
                    candidates.push(Candidate {
                        key: format!("alert:sent:{}:{}", rule.id, key_suffix),
                        content,
                        color: rule.color(),
                        cooldown_secs: rule.cooldown_secs,
                    });
                }
            }
            let generated = candidates.len();

            // 2. Reserver les cles de dedup (SET NX, rapide) : ne restent que
            // les alertes reellement a envoyer dans cette fenetre de cooldown.
            let mut to_send: Vec<(String, u32)> = Vec::new();
            let mut deduplicated = 0usize;
            for candidate in candidates {
                if claim_dedup(redis_conn.clone(), &candidate.key, candidate.cooldown_secs).await {
                    to_send.push((candidate.content, candidate.color));
                } else {
                    deduplicated += 1;
                }
            }

            // 3. Envoyer avec une concurrence bornee (429 respecte dans l'envoi).
            let (sent, errors) = dispatch_webhooks(&client, &webhook, to_send).await;

            if generated > 0 {
                tracing::info!(generated, deduplicated, sent, errors, "alerts: cycle termine");
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
        "auth_failures_1h" => match m.auth_failures_1h {
            Some(value) if rule.triggers(value) => vec![(
                // Cle par heure : ré-alerte au plus une fois par heure meme si le
                // cooldown est plus court.
                chrono::Utc::now().format("%Y%m%d%H").to_string(),
                format!("🚨 **{}** : {:.0} échecs d'auth sur 1h", rule.label, value),
            )],
            _ => vec![],
        },
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
    redis_conn: Option<MultiplexedConnection>,
    container_state: &Option<Arc<RwLock<ContainerMonitorState>>>,
) -> Metrics {
    // ── Ressources host (sysinfo, necessite pid: host cote compose) ──
    // `collect_host_resources` dort 200 ms (deux echantillons CPU) et fait des
    // appels sysinfo bloquants : hors du runtime async via `spawn_blocking`,
    // sinon elle immobilise un thread Tokio a chaque tick. Un panic de la tache
    // retombe sur des mesures nulles, comme une collecte indisponible.
    let (cpu_percent, mem_percent, disk_percent) =
        tokio::task::spawn_blocking(collect_host_resources)
            .await
            .unwrap_or((0.0, 0.0, 0.0));

    // ── Auth failures (1h) ──
    // Le code HTTP vit dans le document JSONB `details`, pas dans une colonne
    // `status_code`, et les logs API portent `category = 'api'` avec la colonne
    // `timestamp` — meme forme que `PgSecurityLogRepository::auth_failures`.
    // L'ancienne requete (`created_at`, `status_code`) echouait a chaque cycle,
    // masquee en 0 par `unwrap_or`, rendant l'alerte `auth_failures_1h` inerte.
    let auth_failures_1h = match sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM logs \
         WHERE category = 'api' \
           AND timestamp > NOW() - INTERVAL '1 hour' \
           AND (details->>'status_code')::int IN (401, 403)",
    )
    .fetch_one(pg_pool)
    .await
    {
        Ok(count) => Some(count as f64),
        Err(error) => {
            tracing::warn!(%error, "alerts: comptage des echecs d'auth impossible");
            None
        }
    };

    // ── TLS expiry (shim host) ──
    let tls_expiry_days = std::fs::read_to_string("/var/lib/sentinel/tls-cert.json")
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("days_until_expiry").and_then(|x| x.as_i64()))
        .map(|d| d as f64);

    // ── Services offline (Redis : bots:known + bot:online:{name}) ──
    let offline_services = collect_offline_services(redis_conn.clone()).await;

    // ── Conteneurs modifies (monitor en memoire) ──
    // Curseur persistant : on n'emet que les changements PLUS RECENTS que le
    // dernier deja traite. Sans lui, un evenement historique encore present dans
    // `recent_changes` (garde 200) etait re-alerte des l'expiration du cooldown
    // Redis. Le curseur survit au redemarrage (Redis) et le timestamp RFC3339
    // est comparable lexicographiquement.
    let mut container_changes = Vec::new();
    if let Some(cs) = container_state {
        let cursor = load_docker_cursor(redis_conn.clone()).await;
        let mut max_ts = cursor.clone();
        let s = cs.read().await;
        for c in &s.recent_changes {
            if !matches!(
                c.kind,
                ContainerChangeKind::Removed | ContainerChangeKind::ImageChanged
            ) {
                continue;
            }
            if c.timestamp <= cursor {
                continue; // deja traite lors d'un cycle precedent
            }
            if c.timestamp > max_ts {
                max_ts = c.timestamp.clone();
            }
            container_changes.push((
                c.container.name.clone(),
                c.kind.as_action().to_owned(),
                c.timestamp.clone(),
            ));
        }
        drop(s);
        if max_ts != cursor {
            save_docker_cursor(redis_conn.clone(), &max_ts).await;
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
///
/// Les `EXISTS` sont PIPELINES en un seul aller-retour au lieu d'un par service.
async fn collect_offline_services(conn: Option<MultiplexedConnection>) -> Vec<String> {
    let Some(mut conn) = conn else {
        return Vec::new();
    };
    let known: Vec<String> = conn.smembers("bots:known").await.unwrap_or_default();
    if known.is_empty() {
        return Vec::new();
    }
    let mut pipe = redis::pipe();
    for name in &known {
        pipe.exists(format!("bot:online:{name}"));
    }
    let online: Vec<bool> = match pipe.query_async(&mut conn).await {
        Ok(states) => states,
        Err(error) => {
            // Fail-open : en cas d'erreur Redis, on ne crie pas au loup.
            tracing::warn!(%error, "alerts: verification des services en ligne impossible");
            return Vec::new();
        }
    };
    known
        .into_iter()
        .zip(online)
        .filter(|(_, is_online)| !is_online)
        .map(|(name, _)| name)
        .collect()
}

/// Cle du curseur des changements Docker deja traites par le dispatcher.
const DOCKER_CURSOR_KEY: &str = "alert:docker:cursor";

/// Dernier timestamp de changement Docker traite (vide si aucun / Redis KO).
async fn load_docker_cursor(conn: Option<MultiplexedConnection>) -> String {
    let Some(mut conn) = conn else {
        return String::new();
    };
    conn.get::<_, Option<String>>(DOCKER_CURSOR_KEY)
        .await
        .unwrap_or(None)
        .unwrap_or_default()
}

/// Avance le curseur. Best-effort : un echec Redis ne fait que risquer une
/// re-alerte au prochain cycle, jamais une alerte manquee.
async fn save_docker_cursor(conn: Option<MultiplexedConnection>, cursor: &str) {
    let Some(mut conn) = conn else {
        return;
    };
    let _: redis::RedisResult<()> = conn.set(DOCKER_CURSOR_KEY, cursor).await;
}

/// Reserve atomiquement l'envoi via une cle Redis `SET NX EX cooldown`.
/// Renvoie `true` si la cle a ete posee (= premier envoi dans la fenetre),
/// `false` si elle existait deja (cooldown en cours). Fail-open si Redis KO
/// (on prefere une eventuelle alerte doublon a une alerte manquee).
async fn claim_dedup(conn: Option<MultiplexedConnection>, key: &str, cooldown_secs: i32) -> bool {
    let Some(mut conn) = conn else {
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

/// Concurrence maximale d'envoi de webhooks. Borne volontairement basse : le
/// webhook Discord est rate-limite, saturer en parallele ne ferait que
/// multiplier les 429.
const MAX_WEBHOOK_CONCURRENCY: usize = 3;

/// Envoie les alertes retenues avec une concurrence bornee. Renvoie
/// `(envoyees, en_erreur)` pour le resume du cycle.
async fn dispatch_webhooks(
    client: &reqwest::Client,
    webhook: &str,
    alerts: Vec<(String, u32)>,
) -> (usize, usize) {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_WEBHOOK_CONCURRENCY));
    let mut tasks = tokio::task::JoinSet::new();
    for (content, color) in alerts {
        // Acquis avant le spawn : borne le nombre d'envois reellement en vol.
        let Ok(permit) = semaphore.clone().acquire_owned().await else {
            break; // semaphore ferme : ne devrait pas arriver
        };
        let client = client.clone();
        let webhook = webhook.to_owned();
        tasks.spawn(async move {
            let _permit = permit; // libere a la fin de l'envoi
            send_webhook(&client, &webhook, &content, color).await
        });
    }

    let (mut sent, mut errors) = (0usize, 0usize);
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(true) => sent += 1,
            _ => errors += 1, // envoi echoue ou tache annulee
        }
    }
    (sent, errors)
}

/// Envoie une alerte sur le webhook Discord. Renvoie `true` si acceptee.
///
/// Un `429` est respecte : on attend le `Retry-After` (borne) et on retente une
/// fois, plutot que de compter l'alerte comme perdue.
async fn send_webhook(client: &reqwest::Client, webhook: &str, content: &str, color: u32) -> bool {
    let body = serde_json::json!({
        "username": "DiscordSentinel · Supervision",
        "embeds": [{
            "title": "Alerte serveur",
            "description": content,
            "color": color,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }]
    });
    // 2 tentatives au plus : l'envoi initial + une reprise apres un 429.
    for attempt in 0..2 {
        match client.post(webhook).json(&body).send().await {
            Ok(r) if r.status().is_success() => return true,
            Ok(r) if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS && attempt == 0 => {
                let retry_after = r
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(1.0)
                    .clamp(0.0, 10.0);
                tracing::warn!(retry_after, "alerte webhook : 429, nouvelle tentative");
                tokio::time::sleep(Duration::from_secs_f64(retry_after)).await;
            }
            Ok(r) => {
                tracing::warn!(status = %r.status(), "alerte webhook : status non-2xx");
                return false;
            }
            Err(e) => {
                tracing::warn!(?e, "alerte webhook : erreur envoi");
                return false;
            }
        }
    }
    false
}

#[cfg(test)]
#[path = "tests/alerts_dispatcher.rs"]
mod tests;
