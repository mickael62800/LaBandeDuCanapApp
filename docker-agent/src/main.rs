//! Agent Docker : le SEUL processus du depot a monter `/var/run/docker.sock`.
//!
//! # Pourquoi il existe
//!
//! Le socket Docker equivaut a un acces root sur l'hote. Il etait monte par
//! `sentinel-api`, c'est-a-dire par le processus qui sert aussi l'OAuth, la
//! moderation Discord et toutes les routes communautaires : la moindre faille
//! dans cette surface donnait l'hote. Cet agent reduit cette surface a un
//! service qui ne sait faire qu'une chose.
//!
//! # Ce qu'il n'est pas
//!
//! Pas de base de donnees, pas de session, pas de notion d'utilisateur, pas de
//! route nginx. Il n'est joignable que depuis le reseau interne Docker, et
//! seulement par les services qui portent le jeton partage.
//!
//! Il ne journalise rien en base non plus : c'est `exploitation-api` qui
//! enregistre QUI a arrete un conteneur. L'agent execute, il ne raconte pas.
//!
//! # Ce qu'il ne resout pas
//!
//! Qui peut appeler l'agent peut toujours creer un conteneur privilegie. Le
//! gain est de reduire la surface qui donne acces au socket, pas de rendre cet
//! acces inoffensif. D'ou la liste blanche : l'agent n'expose QUE les
//! operations du port `DockerHost`, jamais un passe-plat generique vers l'API
//! Docker.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Extension, Path, Query};
use axum::http::{header::AUTHORIZATION, HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use ops_core::domain::entities::docker_host::{
    ContainerSummary, DiskUsage, DockerVersionInfo, ImageSummary, NetworkSummary, PruneOutcome,
    VolumeSummary,
};
use ops_core::ports::outbound::docker_host::DockerHost;
use serde::Deserialize;

mod bollard_host;

use bollard_host::BollardDockerHost;

struct AgentState {
    docker: Arc<dyn DockerHost>,
    token: String,
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Jeton OBLIGATOIRE : demarrer sans authentification exposerait le socket a
    // tout le reseau interne. Mieux vaut refuser de demarrer que tourner ouvert.
    let token = match std::env::var("DOCKER_AGENT_TOKEN") {
        Ok(value) if value.trim().len() >= 16 => value,
        Ok(_) => {
            tracing::error!("DOCKER_AGENT_TOKEN trop court (16 caracteres minimum)");
            std::process::exit(1);
        }
        Err(_) => {
            tracing::error!("DOCKER_AGENT_TOKEN manquant");
            std::process::exit(1);
        }
    };

    let bind: SocketAddr = std::env::var("DOCKER_AGENT_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8095".into())
        .parse()
        .expect("DOCKER_AGENT_BIND_ADDR invalide");

    let state = Arc::new(AgentState {
        docker: Arc::new(BollardDockerHost),
        token,
    });

    let app = router(state);
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .expect("bind impossible");
    tracing::info!(%bind, "docker-agent demarre");
    axum::serve(listener, app).await.expect("serveur arrete");
}

fn router(state: Arc<AgentState>) -> Router {
    Router::new()
        // `/health` reste libre : le healthcheck du conteneur ne porte pas de
        // jeton, et il ne divulgue rien.
        .route("/health", get(|| async { "ok" }))
        .route("/version", get(version))
        .route("/disk-usage", get(disk_usage))
        .route("/containers", get(list_containers))
        .route("/containers/{id}/start", post(start_container))
        .route("/containers/{id}/stop", post(stop_container))
        .route("/containers/{id}/restart", post(restart_container))
        .route("/containers/{id}/remove", post(remove_container))
        .route("/containers/{id}/logs", get(container_logs))
        .route("/images", get(list_images))
        .route("/images/{id}/remove", post(remove_image))
        .route("/volumes", get(list_volumes))
        .route("/volumes/{name}/remove", post(remove_volume))
        .route("/networks", get(list_networks))
        .route("/prune/containers", post(prune_containers))
        .route("/prune/images", post(prune_images))
        .route("/prune/volumes", post(prune_volumes))
        .route("/prune/networks", post(prune_networks))
        .route("/prune/build-cache", post(prune_build_cache))
        .layer(Extension(state))
}

/// Erreur de l'agent. Volontairement avare : les details d'une panne Docker
/// n'ont rien a faire dans une reponse HTTP lisible par un appelant qui ne
/// devrait de toute facon jamais etre un navigateur.
struct AgentError(StatusCode, String);

impl axum::response::IntoResponse for AgentError {
    fn into_response(self) -> axum::response::Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

impl From<ops_core::domain::errors::DomainError> for AgentError {
    fn from(error: ops_core::domain::errors::DomainError) -> Self {
        tracing::warn!(%error, "Operation Docker en echec");
        AgentError(StatusCode::BAD_GATEWAY, error.to_string())
    }
}

fn authorize(headers: &HeaderMap, state: &AgentState) -> Result<(), AgentError> {
    let expected = format!("Bearer {}", state.token);
    let supplied = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok());
    if supplied == Some(expected.as_str()) {
        Ok(())
    } else {
        Err(AgentError(
            StatusCode::UNAUTHORIZED,
            "jeton invalide".to_owned(),
        ))
    }
}

// ── Parametres ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AllQuery {
    #[serde(default)]
    all: bool,
}

#[derive(Deserialize)]
struct StopQuery {
    #[serde(default = "default_timeout")]
    timeout_secs: i64,
}
fn default_timeout() -> i64 {
    30
}

#[derive(Deserialize)]
struct RemoveContainerQuery {
    #[serde(default)]
    force: bool,
    #[serde(default)]
    remove_volumes: bool,
}

#[derive(Deserialize)]
struct RemoveImageQuery {
    #[serde(default)]
    force: bool,
    #[serde(default)]
    no_prune: bool,
}

#[derive(Deserialize)]
struct ForceQuery {
    #[serde(default)]
    force: bool,
}

#[derive(Deserialize)]
struct LogsQuery {
    #[serde(default = "default_tail")]
    tail: u32,
    #[serde(default)]
    timestamps: bool,
}
fn default_tail() -> u32 {
    200
}

// ── Handlers ──────────────────────────────────────────────────────────────

macro_rules! guarded {
    ($state:ident, $headers:ident) => {
        authorize(&$headers, &$state)?
    };
}

async fn version(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<Json<DockerVersionInfo>, AgentError> {
    guarded!(state, headers);
    Ok(Json(state.docker.version_info().await?))
}

async fn disk_usage(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<Json<DiskUsage>, AgentError> {
    guarded!(state, headers);
    Ok(Json(state.docker.disk_usage().await?))
}

async fn list_containers(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Query(q): Query<AllQuery>,
) -> Result<Json<Vec<ContainerSummary>>, AgentError> {
    guarded!(state, headers);
    Ok(Json(state.docker.list_containers(q.all).await?))
}

async fn start_container(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, AgentError> {
    guarded!(state, headers);
    state.docker.start_container(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn stop_container(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<StopQuery>,
) -> Result<StatusCode, AgentError> {
    guarded!(state, headers);
    state.docker.stop_container(&id, q.timeout_secs).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn restart_container(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<StopQuery>,
) -> Result<StatusCode, AgentError> {
    guarded!(state, headers);
    state.docker.restart_container(&id, q.timeout_secs).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_container(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<RemoveContainerQuery>,
) -> Result<StatusCode, AgentError> {
    guarded!(state, headers);
    state
        .docker
        .remove_container(&id, q.force, q.remove_volumes)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn container_logs(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<LogsQuery>,
) -> Result<String, AgentError> {
    guarded!(state, headers);
    Ok(state
        .docker
        .container_logs(&id, q.tail, q.timestamps)
        .await?)
}

async fn list_images(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ImageSummary>>, AgentError> {
    guarded!(state, headers);
    Ok(Json(state.docker.list_images().await?))
}

async fn remove_image(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<RemoveImageQuery>,
) -> Result<StatusCode, AgentError> {
    guarded!(state, headers);
    state.docker.remove_image(&id, q.force, q.no_prune).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_volumes(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<VolumeSummary>>, AgentError> {
    guarded!(state, headers);
    Ok(Json(state.docker.list_volumes().await?))
}

async fn remove_volume(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Query(q): Query<ForceQuery>,
) -> Result<StatusCode, AgentError> {
    guarded!(state, headers);
    state.docker.remove_volume(&name, q.force).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_networks(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<NetworkSummary>>, AgentError> {
    guarded!(state, headers);
    Ok(Json(state.docker.list_networks().await?))
}

async fn prune_containers(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<Json<PruneOutcome>, AgentError> {
    guarded!(state, headers);
    Ok(Json(state.docker.prune_containers().await?))
}

async fn prune_images(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Query(q): Query<AllQuery>,
) -> Result<Json<PruneOutcome>, AgentError> {
    guarded!(state, headers);
    Ok(Json(state.docker.prune_images(q.all).await?))
}

async fn prune_volumes(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<Json<PruneOutcome>, AgentError> {
    guarded!(state, headers);
    Ok(Json(state.docker.prune_volumes().await?))
}

async fn prune_networks(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<Json<PruneOutcome>, AgentError> {
    guarded!(state, headers);
    Ok(Json(state.docker.prune_networks().await?))
}

async fn prune_build_cache(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Query(q): Query<AllQuery>,
) -> Result<Json<PruneOutcome>, AgentError> {
    guarded!(state, headers);
    Ok(Json(state.docker.prune_build_cache(q.all).await?))
}
