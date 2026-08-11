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
pub mod http_job;
pub mod metrics;
pub mod redis_helpers;

use std::any::Any;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::stream::{FuturesUnordered, StreamExt};
use futures_util::FutureExt;
use platform_common::config_flags::parse_bool_str;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::signal;
use tokio::task::{AbortHandle, JoinHandle};
use tracing::{error, info, warn};

// ── Constantes ──

/// Nombre max de connexions PostgreSQL par defaut.
const DEFAULT_PG_MAX_CONNECTIONS: u32 = 5;
/// Timeout d'acquisition de connexion PostgreSQL par defaut (secondes).
const DEFAULT_PG_ACQUIRE_TIMEOUT_SECS: u64 = 5;
/// Intervalle de heartbeat par defaut (secondes).
const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// Handle nomme d'une tache de fond conservee par le processus appelant.
pub struct SupervisedTask {
    name: &'static str,
    handle: JoinHandle<()>,
}

impl SupervisedTask {
    pub fn spawn<F>(name: &'static str, future: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Self {
            name,
            handle: tokio::spawn(future),
        }
    }

    fn abort_handle(&self) -> AbortHandle {
        self.handle.abort_handle()
    }
}

/// Bilan de l'attente des taches pendant un arret gracieux.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ShutdownReport {
    pub completed: usize,
    pub aborted: usize,
    pub join_errors: usize,
}

/// Attend toutes les taches avec une echeance globale, puis annule celles qui
/// depassent cette echeance. Le pool et les autres ressources peuvent etre
/// fermes seulement apres le retour de cette fonction.
pub async fn wait_for_tasks(tasks: Vec<SupervisedTask>, timeout: Duration) -> ShutdownReport {
    let abort_handles: Vec<AbortHandle> = tasks.iter().map(SupervisedTask::abort_handle).collect();
    let mut pending: FuturesUnordered<_> = tasks
        .into_iter()
        .map(|task| async move { (task.name, task.handle.await) })
        .collect();
    let mut report = ShutdownReport::default();
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            _ = &mut deadline, if !pending.is_empty() => {
                report.aborted = pending.len();
                warn!(pending = report.aborted, "Echeance d'arret atteinte, annulation des taches restantes");
                for handle in &abort_handles {
                    handle.abort();
                }
                while let Some((name, result)) = pending.next().await {
                    if let Err(error) = result {
                        if !error.is_cancelled() {
                            report.join_errors += 1;
                            error!(task = name, %error, "Tache terminee anormalement pendant l'arret");
                        }
                    }
                }
                break;
            }
            result = pending.next() => {
                let Some((name, result)) = result else { break };
                match result {
                    Ok(()) => report.completed += 1,
                    Err(error) => {
                        report.join_errors += 1;
                        error!(task = name, %error, "Tache terminee anormalement");
                    }
                }
            }
        }
    }

    report
}

/// Metriques communes d'un job, y compris les boucles specialisees qui ne
/// passent pas directement par `spawn_periodic`.
pub struct JobMetrics {
    job: &'static str,
    worker: &'static str,
    consecutive_errors: u64,
}

impl JobMetrics {
    pub fn new(job: &'static str, worker: &'static str) -> Self {
        ::metrics::gauge!("worker_job_alive", "job" => job, "worker" => worker).set(1.0);
        ::metrics::gauge!("worker_job_consecutive_errors", "job" => job, "worker" => worker)
            .set(0.0);
        Self {
            job,
            worker,
            consecutive_errors: 0,
        }
    }

    pub fn started(&self) {
        ::metrics::gauge!("worker_job_last_start_timestamp_seconds", "job" => self.job, "worker" => self.worker)
            .set(unix_timestamp_seconds());
    }

    pub fn succeeded(&mut self, duration: Duration) {
        self.consecutive_errors = 0;
        ::metrics::gauge!("worker_job_last_success_timestamp_seconds", "job" => self.job, "worker" => self.worker)
            .set(unix_timestamp_seconds());
        self.record_duration(duration);
        self.record_consecutive_errors();
    }

    pub fn failed(&mut self, duration: Duration) {
        self.consecutive_errors = self.consecutive_errors.saturating_add(1);
        self.record_duration(duration);
        self.record_consecutive_errors();
        ::metrics::counter!("worker_job_errors_total", "job" => self.job, "worker" => self.worker)
            .increment(1);
    }

    pub fn panicked(&mut self, duration: Duration) {
        self.failed(duration);
        ::metrics::counter!("worker_job_panics_total", "job" => self.job, "worker" => self.worker)
            .increment(1);
    }

    pub fn stopped(&self) {
        ::metrics::gauge!("worker_job_alive", "job" => self.job, "worker" => self.worker).set(0.0);
    }

    fn record_duration(&self, duration: Duration) {
        ::metrics::gauge!("worker_job_last_duration_seconds", "job" => self.job, "worker" => self.worker)
            .set(duration.as_secs_f64());
    }

    fn record_consecutive_errors(&self) {
        ::metrics::gauge!("worker_job_consecutive_errors", "job" => self.job, "worker" => self.worker)
            .set(self.consecutive_errors as f64);
    }
}

fn unix_timestamp_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

/// Lit un entier positif depuis l'environnement, avec warning et valeur par
/// defaut si la variable est invalide.
pub fn env_u64(name: &str, default: u64) -> u64 {
    match std::env::var(name) {
        Ok(value) => value.parse().unwrap_or_else(|_| {
            warn!(var = name, %value, default, "valeur invalide, defaut applique");
            default
        }),
        Err(_) => default,
    }
}

/// Lance une fonction immediatement puis a intervalle fixe, sans chevauchement.
pub fn spawn_interval<F, Fut>(name: &'static str, interval_secs: u64, mut task: F)
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
{
    info!(task = name, interval_secs, "Tache periodique planifiee");
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs.max(1)));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let started = std::time::Instant::now();
            match task().await {
                Ok(()) => tracing::debug!(
                    task = name,
                    elapsed_ms = started.elapsed().as_millis(),
                    "Tick termine"
                ),
                Err(error) => error!(task = name, %error, "Tick en echec"),
            }
        }
    });
}

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
///
/// Meme route et meme garde que `send_worker_log` : sans collecteur declare par
/// la plateforme, on ne poste pas.
pub async fn send_lifecycle_log(api_url: &str, worker_name: &str, level: &str, message: &str) {
    send_worker_log(
        api_url,
        worker_name,
        level,
        "lifecycle",
        message,
        serde_json::json!({ "event_type": "worker.lifecycle" }),
    )
    .await;
}

// ── Heartbeat ──

/// Demarre un heartbeat periodique vers l'API.
///
/// La route `/api/bots/heartbeat` cote API est protegee par l'auth_middleware,
/// on doit donc envoyer l'`API_KEY` en header `Authorization: Bearer` — sinon
/// l'API retourne 401 silencieusement (reqwest::send() considere un 401 comme
/// un succes reseau, donc le worker ne log meme pas l'erreur). L'API_KEY est
/// lue depuis l'env au demarrage du heartbeat.
pub fn start_heartbeat(
    client: reqwest::Client,
    api_url: String,
    worker_name: &'static str,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> SupervisedTask {
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

    SupervisedTask::spawn("heartbeat", async move {
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

            tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(interval.max(1))) => {}
            }
        }
    })
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

    platform_common::config_flags::parse_enabled_flag(result.as_deref())
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

/// Cle d'API du push HTTP des logs de jobs, posee une fois au demarrage.
///
/// Non initialisee = la plateforme n'expose pas de collecteur : `send_worker_log`
/// s'en tient alors au log local. Avant, la cle etait lue en dur dans
/// `SENTINEL_API_KEY` : `nexus-worker` et `atrium-worker`, qui ne l'ont pas dans
/// leur environnement, POSTaient donc sans authentification sur une route
/// (`/api/logs`) que leur API n'expose meme pas. Une requete par tick, un 404,
/// et l'echec avale en `debug!` — invisible.
static LOG_API_KEY: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
static LOG_HTTP_CLIENT: std::sync::OnceLock<http_job::HttpJobClient> = std::sync::OnceLock::new();

/// Active le push HTTP des logs de jobs vers l'API de la plateforme.
///
/// A appeler dans le `main` du worker, avant `spawn_periodic`. Une plateforme
/// dont l'API n'a pas de route `POST /api/logs` ne l'appelle pas : ses jobs
/// restent traces localement (stdout, donc Docker/Grafana), sans requete
/// perdue.
pub fn enable_worker_log_push(api_key: impl Into<String>) {
    let _ = LOG_API_KEY.set(Some(api_key.into()));
}

/// Envoie un log structure d'execution de tache vers l'API (categorie "worker").
/// Helper public reutilisable pour loguer du contexte applicatif depuis un job.
///
/// Le log local est toujours emis ; le POST vers l'API n'a lieu que si la
/// plateforme l'a active via `enable_worker_log_push`.
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
    // Pas de collecteur configure pour cette plateforme : le log local
    // ci-dessus est tout ce qui est attendu, on ne fabrique pas de requete.
    let Some(api_key) = LOG_API_KEY.get().and_then(|k| k.as_deref()) else {
        return;
    };
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

    let payload = serde_json::json!({
        "level": level,
        "bot": worker_name,
        "message": message,
        "category": "worker",
        "details": serde_json::Value::Object(details_obj),
    });
    let client = LOG_HTTP_CLIENT.get_or_init(|| {
        http_job::HttpJobClient::new(
            api_url.to_owned(),
            api_key.to_owned(),
            Duration::from_secs(10),
        )
    });
    if let Err(e) = client.post_json_unit("/api/logs", &payload).await {
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
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    api_url: String,
    worker_name: &'static str,
    task_fn: F,
) -> SupervisedTask
where
    F: Fn(
            PgPool,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
        + Send
        + Sync
        + 'static,
{
    info!(task = name, interval_secs, "Tache periodique planifiee");

    SupervisedTask::spawn(name, async move {
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

        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs.max(1)));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let slow_threshold = Duration::from_secs(5);
        let mut tick_count: u64 = 0;
        let mut job_metrics = JobMetrics::new(name, worker_name);

        loop {
            tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                    continue;
                }
                _ = interval.tick() => {}
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
            job_metrics.started();
            let result = run_caught_task(|| task_fn(pool.clone())).await;
            let elapsed = start.elapsed();
            let elapsed_ms = elapsed.as_millis() as u64;

            match result {
                TaskOutcome::Success => {
                    job_metrics.succeeded(elapsed);
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
                TaskOutcome::Error(e) => {
                    job_metrics.failed(elapsed);
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
                TaskOutcome::Panicked(panic) => {
                    job_metrics.panicked(elapsed);
                    error!(task = name, %panic, elapsed_ms, "Panic capturee dans la tache periodique");
                    send_worker_log(
                        &api_url,
                        worker_name,
                        "error",
                        name,
                        &format!("Panic job {} : {}", name, panic),
                        serde_json::json!({
                            "panic": panic,
                            "elapsed_ms": elapsed_ms,
                            "tick": tick_count,
                            "event_type": "job.panic",
                            "policy": "restart_next_tick",
                        }),
                    )
                    .await;
                }
            }
        }

        job_metrics.stopped();
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
    })
}

#[derive(Debug, PartialEq, Eq)]
enum TaskOutcome {
    Success,
    Error(String),
    Panicked(String),
}

async fn run_caught_task<F, Fut>(task: F) -> TaskOutcome
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    match AssertUnwindSafe(async move { task().await })
        .catch_unwind()
        .await
    {
        Ok(Ok(())) => TaskOutcome::Success,
        Ok(Err(error)) => TaskOutcome::Error(error),
        Err(payload) => TaskOutcome::Panicked(panic_message(payload.as_ref())),
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "panic sans message".to_owned()
    }
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

    #[tokio::test]
    async fn supervised_tasks_finish_before_the_deadline() {
        let tasks = vec![SupervisedTask::spawn("short_job", async {})];

        let report = wait_for_tasks(tasks, Duration::from_secs(1)).await;

        assert_eq!(
            report,
            ShutdownReport {
                completed: 1,
                aborted: 0,
                join_errors: 0,
            }
        );
    }

    #[tokio::test]
    async fn supervised_tasks_are_aborted_after_the_deadline() {
        let tasks = vec![SupervisedTask::spawn("long_job", async {
            std::future::pending::<()>().await;
        })];

        let report = wait_for_tasks(tasks, Duration::from_millis(10)).await;

        assert_eq!(report.completed, 0);
        assert_eq!(report.aborted, 1);
        assert_eq!(report.join_errors, 0);
    }

    #[tokio::test]
    async fn periodic_task_panics_are_captured() {
        let outcome = run_caught_task(|| async {
            panic!("job exploded");
            #[allow(unreachable_code)]
            Ok(())
        })
        .await;

        assert_eq!(outcome, TaskOutcome::Panicked("job exploded".to_owned()));
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
