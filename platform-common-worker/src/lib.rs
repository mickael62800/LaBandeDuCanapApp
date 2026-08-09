//! Infrastructure partagee entre les workers DiscordSentinel.
//!
//! Elimine la duplication de : shutdown signal, lifecycle logging,
//! heartbeat, scheduler, pool creation, observabilité Prometheus,
//! appels HTTP/gRPC vers l'API, init Redis client.

// Tout ce qui est ici est appele par au moins un job. Les helpers gardes
// "pour un futur job" (get_json, bearer_interceptor, with_bearer,
// is_feature_enabled, config_or_env_bool) ont ete supprimes : ils vivaient
// derriere un `#![allow(dead_code)]` global qui masquait aussi les vrais
// oublis. Un helper dont on a besoin se réécrit en quelques minutes ; un
// helper mort se maintient indéfiniment.

pub mod api;
pub mod grpc;
pub mod metrics;
pub mod redis_helpers;

use std::time::Duration;

pub use sentinel_core::domain::entities::system::config_parsers::parse_bool_str;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::signal;
use tracing::{error, info, warn};

// ── Constantes ──

/// Nombre max de connexions PostgreSQL par defaut.
const DEFAULT_PG_MAX_CONNECTIONS: u32 = 5;
/// Timeout d'acquisition de connexion PostgreSQL par defaut (secondes).
const DEFAULT_PG_ACQUIRE_TIMEOUT_SECS: u64 = 5;
/// Intervalle de heartbeat par defaut (secondes).
const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 30;

// ── Init ──

/// Initialise dotenvy + tracing avec un filtre par defaut.
///
/// Le serveur metrics (`/metrics` sur METRICS_PORT, Prometheus) doit
/// etre demarre explicitement par le main via
/// `common::metrics::init_observability(WORKER_NAME)`. Avant la fusion
/// `init_tracing` le faisait automatiquement, mais ca causait un
/// double-bind quand le main appelait aussi explicitement
/// init_observability (port 9100 deja pris -> erreur au boot).
pub fn init_tracing(default_filter: &str) {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| default_filter.into()),
        )
        .init();
}

// ── PostgreSQL ──

/// Cree un pool PostgreSQL avec des parametres configurables via env.
/// Retourne une erreur au lieu de panic si la connexion echoue.
pub async fn create_pg_pool(database_url: &str) -> PgPool {
    let max_connections: u32 = std::env::var("PG_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PG_MAX_CONNECTIONS);

    let acquire_timeout: u64 = std::env::var("PG_ACQUIRE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PG_ACQUIRE_TIMEOUT_SECS);

    match PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(acquire_timeout))
        .connect(database_url)
        .await
    {
        Ok(pool) => pool,
        Err(e) => {
            error!(error = %e, "Impossible de se connecter a PostgreSQL");
            std::process::exit(1);
        }
    }
}

// ── Lifecycle Logging ──

/// Envoie un log de cycle de vie a l'API.
pub async fn send_lifecycle_log(api_url: &str, worker_name: &str, level: &str, message: &str) {
    let api_key = std::env::var("SENTINEL_API_KEY").unwrap_or_default();
    let mut req = reqwest::Client::new()
        .post(format!("{}/api/logs", api_url))
        .json(&serde_json::json!({
            "level": level,
            "bot": worker_name,
            "server": "",
            "message": message,
            "category": "worker",
        }));
    if !api_key.is_empty() {
        req = req.bearer_auth(&api_key);
    }
    if let Err(e) = req.send().await {
        warn!(error = %e, worker = worker_name, "Erreur envoi log lifecycle");
    }
}

// ── Heartbeat ──

/// Demarre un heartbeat periodique vers l'API.
///
/// La route `/api/bots/heartbeat` cote API est protegee par l'auth_middleware,
/// on doit donc envoyer l'`API_KEY` en header `Authorization: Bearer` — sinon
/// l'API retourne 401 silencieusement (reqwest::send() considere un 401 comme
/// un succes reseau, donc le worker ne log meme pas l'erreur). L'API_KEY est
/// lue depuis l'env au demarrage du heartbeat.
pub fn start_heartbeat(api_url: String, worker_name: &'static str) {
    let interval: u64 = std::env::var("HEARTBEAT_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_HEARTBEAT_INTERVAL_SECS);

    let api_key = std::env::var("SENTINEL_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        warn!(
            worker = worker_name,
            "API_KEY non definie — les heartbeats seront rejetes avec 401"
        );
    }

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let url = format!("{}/api/bots/heartbeat", api_url);

        loop {
            let req = client
                .post(&url)
                .bearer_auth(&api_key)
                .json(&serde_json::json!({ "name": worker_name }));

            match req.send().await {
                Ok(resp) if resp.status().is_success() => {}
                Ok(resp) => {
                    warn!(
                        status = %resp.status(),
                        worker = worker_name,
                        "Heartbeat rejete par l'API"
                    );
                }
                Err(e) => {
                    warn!(error = %e, worker = worker_name, "Heartbeat echoue");
                }
            }

            tokio::time::sleep(Duration::from_secs(interval)).await;
        }
    });
}

// ── Shutdown Signal ──

/// Attend un signal d'arret (Ctrl+C ou SIGTERM).
pub async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = signal::ctrl_c().await {
            error!(error = %e, "Impossible d'ecouter Ctrl+C");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => error!(error = %e, "Impossible d'ecouter SIGTERM"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Signal Ctrl+C recu"),
        _ = terminate => info!("Signal SIGTERM recu"),
    }
}

// ── Worker Enabled Check ──

/// Verifie si un worker est active pour une guild donnee.
/// Fail-closed : sans ligne `enabled` explicite, le worker ne tourne pas.
pub async fn is_worker_enabled(pool: &PgPool, guild_id: &str, worker_name: &str) -> bool {
    let result: Option<String> = sqlx::query_scalar(
        "SELECT config_value FROM bot_guild_config \
         WHERE guild_id = $1 AND bot_name = $2 AND config_key = 'enabled'",
    )
    .bind(guild_id)
    .bind(worker_name)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    sentinel_core::domain::entities::system::config_parsers::parse_enabled_flag(result.as_deref())
}

/// Verifie si le worker est active pour au moins une guild.
/// Retourne true si:
/// - Aucune entree `enabled` trouvee (defaut = active)
/// - Au moins une entree `enabled = true`
///
/// Retourne false si toutes les entrees trouvees sont `enabled = false`.
pub async fn is_worker_globally_enabled(pool: &PgPool, worker_name: &str) -> bool {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT config_value FROM bot_guild_config \
         WHERE bot_name = $1 AND config_key = 'enabled'",
    )
    .bind(worker_name)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return true; // Pas de config = active par defaut
    }
    rows.iter().any(|(v,)| parse_bool_str(v))
}

/// Constantes de temps utilitaires.
pub const SECS_PER_MINUTE: u64 = 60;
pub const SECS_PER_HOUR: u64 = 3600;

// ── Config Helpers ──

/// Charge DATABASE_URL depuis l'environnement. Exit si absent.
pub fn load_database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        error!("DATABASE_URL non defini");
        std::process::exit(1);
    })
}

/// Charge API_URL depuis l'environnement avec fallback localhost.
pub fn load_api_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:3000".into())
}

/// Charge REDIS_URL depuis l'environnement avec fallback localhost.
pub fn load_redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into())
}

// ── DB Config Loading ──

/// Modules de worker — un par dossier `src/domains/{domain}/`.
/// Le `bot_name` dans `bot_guild_config` utilise ces noms pour stocker
/// la config de chaque domaine. La migration 204 renomme les anciens
/// noms d'infrastructure (`cleanup-worker`, etc.) vers ces noms.
const WORKER_MODULES: &[&str] = &[
    "ai",
    "analytics",
    "announcements",
    "audit-bot",
    "automod-bot",
    "cache",
    "cleanup",
    "export",
    // Domaines dont les jobs lisent leur config sous ces bot_name (bug corrigé :
    // ils étaient absents, leurs intervalles n'étaient jamais résolus depuis la
    // config DB de leur domaine — seulement sous `sentinel-worker`).
    "guild-backup-bot",
    "welcome-bot",
    "moderation-bot",
    "monitoring",
    "security-bot",
    "temp_roles",
    "ticket-bot",
];

/// Charge toute la config workers depuis `bot_guild_config`.
///
/// Lit les rows ou `bot_name` est :
///   - le nom du processus (`sentinel-worker`) pour la config globale,
///   - un nom de module (`cleanup`, `ticket-bot`, ...) pour la config
///     specifique a un domaine.
///
/// Priorite : nom du processus d'abord, puis modules. Decouple le
/// nom de processus du nom de configuration : si on renomme le
/// binaire plus tard, les configs ne bougent pas.
///
/// La migration 204 a converti les anciens noms (`cleanup-worker`,
/// `ticket-bot`, etc.) vers ces noms de modules.
pub async fn load_worker_config(
    pool: &PgPool,
    worker_name: &str,
) -> std::collections::HashMap<String, String> {
    let mut all_names: Vec<&str> = Vec::with_capacity(1 + WORKER_MODULES.len());
    all_names.push(worker_name);
    all_names.extend_from_slice(WORKER_MODULES);

    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT bot_name, config_key, config_value \
         FROM bot_guild_config \
         WHERE bot_name = ANY($1) \
         ORDER BY \
            CASE WHEN bot_name = $2 THEN 0 ELSE 1 END, \
            updated_at DESC",
    )
    .bind(&all_names)
    .bind(worker_name)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut map = std::collections::HashMap::new();
    for (_bot_name, key, value) in rows {
        // entry().or_insert() preserve la 1ere occurrence -> la priorite
        // est definie par l'ORDER BY (sentinel-worker d'abord).
        map.entry(key).or_insert(value);
    }
    map
}

/// Lit une valeur depuis la config DB, sinon env var, sinon defaut.
pub fn config_or_env<T: std::str::FromStr>(
    db_config: &std::collections::HashMap<String, String>,
    db_key: &str,
    env_key: &str,
    default: T,
) -> T {
    // Priorite 1 : config DB
    if let Some(val) = db_config.get(db_key) {
        if let Ok(parsed) = val.parse() {
            return parsed;
        }
    }
    // Priorite 2 : env var
    if let Ok(val) = std::env::var(env_key) {
        if let Ok(parsed) = val.parse() {
            return parsed;
        }
    }
    // Priorite 3 : defaut
    default
}

/// Charge une variable d'environnement avec un fallback par defaut.
pub fn load_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Charge une variable d'environnement booleenne (accepte "true"/"1"/"yes").
pub fn load_env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => parse_bool_str(&v),
        Err(_) => default,
    }
}

// ── Periodic Scheduler ──

/// Envoie un log structure d'execution de tache vers l'API (categorie "worker").
/// Helper public reutilisable pour loguer du contexte applicatif depuis un job.
pub async fn send_worker_log(
    api_url: &str,
    worker_name: &str,
    level: &str,
    job_name: &str,
    message: &str,
    details: serde_json::Value,
) {
    match level {
        "error" => tracing::error!(worker = worker_name, job = job_name, ?details, "{message}"),
        "warn" => tracing::warn!(worker = worker_name, job = job_name, ?details, "{message}"),
        _ => tracing::info!(worker = worker_name, job = job_name, ?details, "{message}"),
    }
    let api_key = std::env::var("SENTINEL_API_KEY").unwrap_or_default();
    let client = reqwest::Client::new();
    // Merge job dans les details pour retrouver facilement
    let mut details_obj = match details {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };
    details_obj.insert(
        "job".to_string(),
        serde_json::Value::String(job_name.to_string()),
    );
    details_obj.insert(
        "event_type".to_string(),
        serde_json::Value::String(format!("job.{}", level)),
    );

    let mut req = client
        .post(format!("{}/api/logs", api_url))
        .json(&serde_json::json!({
            "level": level,
            "bot": worker_name,
            "message": message,
            "category": "worker",
            "details": serde_json::Value::Object(details_obj),
        }));
    if !api_key.is_empty() {
        req = req.bearer_auth(&api_key);
    }
    if let Err(e) = req.send().await {
        tracing::debug!(error = %e, worker = worker_name, "send_worker_log echec");
    }
}

/// Lance une tache periodique avec gestion du shutdown et reporting d'erreurs.
///
/// Logs envoyes a l'API (categorie worker) :
/// - 1 log "info" au boot (lifecycle)
/// - 1 log "info" a chaque tick reussi avec duree + interval (`event_type: job.success`)
/// - 1 log "warn" si la duree depasse 5s (job lent)
/// - 1 log "error" sur Err(e) du job (`event_type: job.error`)
pub fn spawn_periodic<F>(
    name: &'static str,
    interval_secs: u64,
    pool: PgPool,
    shutdown: tokio::sync::watch::Receiver<bool>,
    api_url: String,
    worker_name: &'static str,
    task_fn: F,
) where
    F: Fn(
            PgPool,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
        + Send
        + 'static,
{
    info!(task = name, interval_secs, "Tache periodique planifiee");

    tokio::spawn(async move {
        // Log boot (info) — confirme cote API que ce job tourne effectivement
        send_worker_log(
            &api_url,
            worker_name,
            "info",
            name,
            &format!("Job {} planifie (intervalle {}s)", name, interval_secs),
            serde_json::json!({ "interval_secs": interval_secs, "event_type": "job.scheduled" }),
        )
        .await;

        let interval = Duration::from_secs(interval_secs);
        let slow_threshold = Duration::from_secs(5);
        let mut tick_count: u64 = 0;

        loop {
            tokio::time::sleep(interval).await;

            if *shutdown.borrow() {
                info!(task = name, "Tache periodique arretee (shutdown)");
                send_worker_log(
                    &api_url,
                    worker_name,
                    "info",
                    name,
                    &format!("Job {} arrete (shutdown)", name),
                    serde_json::json!({ "ticks": tick_count, "event_type": "job.stopped" }),
                )
                .await;
                break;
            }

            // Verifie le flag enabled en DB avant chaque tick.
            if !is_worker_globally_enabled(&pool, worker_name).await {
                tracing::debug!(
                    task = name,
                    worker = worker_name,
                    "Worker desactive via config, skip tick"
                );
                continue;
            }

            tick_count += 1;
            let start = std::time::Instant::now();
            let result = task_fn(pool.clone()).await;
            let elapsed = start.elapsed();
            let elapsed_ms = elapsed.as_millis() as u64;

            match result {
                Ok(()) => {
                    let level = if elapsed > slow_threshold {
                        "warn"
                    } else {
                        "info"
                    };
                    let msg = if elapsed > slow_threshold {
                        format!("Job {} lent ({} ms)", name, elapsed_ms)
                    } else {
                        format!("Job {} ok ({} ms)", name, elapsed_ms)
                    };
                    send_worker_log(
                        &api_url, worker_name, level, name, &msg,
                        serde_json::json!({
                            "elapsed_ms": elapsed_ms,
                            "tick": tick_count,
                            "event_type": if elapsed > slow_threshold { "job.slow" } else { "job.success" },
                        }),
                    ).await;
                }
                Err(e) => {
                    error!(task = name, error = %e, elapsed_ms, "Erreur tache periodique");
                    send_worker_log(
                        &api_url,
                        worker_name,
                        "error",
                        name,
                        &format!("Erreur job {} : {}", name, e),
                        serde_json::json!({
                            "error": e.to_string(),
                            "elapsed_ms": elapsed_ms,
                            "tick": tick_count,
                            "event_type": "job.error",
                        }),
                    )
                    .await;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_constants_are_reasonable() {
        assert_eq!(DEFAULT_PG_MAX_CONNECTIONS, 5);
        assert_eq!(DEFAULT_PG_ACQUIRE_TIMEOUT_SECS, 5);
        assert_eq!(DEFAULT_HEARTBEAT_INTERVAL_SECS, 30);
        assert_eq!(SECS_PER_MINUTE, 60);
        assert_eq!(SECS_PER_HOUR, 3600);
    }

    #[test]
    fn time_constants_coherent() {
        assert_eq!(SECS_PER_HOUR, SECS_PER_MINUTE * 60);
    }

    // ── config_or_env tests ──

    #[test]
    fn config_or_env_db_takes_priority() {
        let mut db = std::collections::HashMap::new();
        db.insert("my_key".into(), "42".into());
        let result: u64 = config_or_env(&db, "my_key", "NONEXISTENT_ENV_VAR_XYZ", 99);
        assert_eq!(result, 42);
    }

    #[test]
    fn config_or_env_falls_back_to_default() {
        let db = std::collections::HashMap::new();
        let result: u64 = config_or_env(&db, "missing", "NONEXISTENT_ENV_VAR_XYZ", 99);
        assert_eq!(result, 99);
    }

    #[test]
    fn config_or_env_invalid_db_value_falls_back() {
        let mut db = std::collections::HashMap::new();
        db.insert("key".into(), "not_a_number".into());
        let result: u64 = config_or_env(&db, "key", "NONEXISTENT_ENV_VAR_XYZ", 50);
        assert_eq!(result, 50);
    }
}

