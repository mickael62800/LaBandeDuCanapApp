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
//! seulement par les services qui portent le jeton de la surface visee.
//!
//! # Deux surfaces, deux jetons
//!
//! - **Hote** (`/containers`, `/prune/*`, `/images`, …) — `DOCKER_AGENT_TOKEN`,
//!   porte par `ops-api`.
//! - **Serveurs de jeu** (`/game/*`) — `DOCKER_AGENT_GAME_TOKEN`, porte par
//!   `nexus-api`.
//!
//! Un jeton unique donnait a Nexus le pouvoir d'arreter ou de purger n'importe
//! quel conteneur de l'hote, `postgres` et `auth-api` compris : on lui avait
//! retire l'acces direct au socket pour lui confier la cle du processus qui
//! l'a. La separation est stricte dans les deux sens.
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
use ops_core::domain::entities::game_runtime::{
    ContainerSpec, ContainerStats, ContainerStatus, ManagedContainer,
};
use ops_core::ports::outbound::docker_host::DockerHost;
use ops_core::ports::outbound::game_runtime::GameContainerRuntime;
use serde::Deserialize;
use subtle::ConstantTimeEq;

mod bollard_game;
mod bollard_host;

use bollard_game::{make_docker_client, DockerContainerRuntime};
use bollard_host::BollardDockerHost;

struct AgentState {
    docker: Arc<dyn DockerHost>,
    /// Cycle de vie des conteneurs de jeu (Nexus). `None` si le socket Docker
    /// n'a pas pu etre ouvert : l'agent continue de servir la surface
    /// `DockerHost` (qui ouvre sa propre connexion) et repond 503 sur `/game`,
    /// ce qui laisse nexus-api refuser proprement une creation au lieu de
    /// fabriquer un serveur qui ne demarrera jamais.
    game: Option<Arc<dyn GameContainerRuntime>>,
    /// Jeton de la surface d'ADMINISTRATION DE L'HOTE (`/containers`,
    /// `/prune/*`, `/images`, …). Porte par `ops-api`.
    host_token: String,
    /// Jeton de la surface SERVEURS DE JEU (`/game/*`). Porte par `nexus-api`.
    ///
    /// # Pourquoi deux jetons et pas un
    ///
    /// Avec un jeton unique, les identifiants de `nexus-api` ouvraient aussi
    /// l'administration de l'hote : arreter ou supprimer n'importe quel
    /// conteneur, `postgres` et `auth-api` compris. On avait retire a Nexus
    /// l'acces direct au socket pour lui donner la cle du processus qui l'a.
    ///
    /// La separation est STRICTE dans les deux sens : le jeton hote n'ouvre pas
    /// `/game/*`. `ops-api` n'y a aucune raison d'aller, et un jeton qui ouvre
    /// tout finit toujours par etre celui qu'on distribue partout.
    game_token: String,
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Jetons OBLIGATOIRES : demarrer sans authentification exposerait le socket
    // a tout le reseau interne. Mieux vaut refuser de demarrer que tourner
    // ouvert. Pas de repli du jeton jeu sur le jeton hote : un repli
    // silencieux rouvrirait exactement le trou que cette separation ferme.
    let host_token = required_token("DOCKER_AGENT_TOKEN");
    let game_token = required_token("DOCKER_AGENT_GAME_TOKEN");

    if host_token == game_token {
        tracing::error!(
            "DOCKER_AGENT_TOKEN et DOCKER_AGENT_GAME_TOKEN sont identiques : \
             la separation des surfaces ne protege alors rien"
        );
        std::process::exit(1);
    }

    let bind: SocketAddr = std::env::var("DOCKER_AGENT_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8095".into())
        .parse()
        .expect("DOCKER_AGENT_BIND_ADDR invalide");

    let game: Option<Arc<dyn GameContainerRuntime>> = match make_docker_client() {
        Ok(client) => Some(Arc::new(DockerContainerRuntime::new(client))),
        Err(error) => {
            tracing::warn!(%error, "socket Docker indisponible : surface /game desactivee");
            None
        }
    };

    let state = Arc::new(AgentState {
        docker: Arc::new(BollardDockerHost),
        game,
        host_token,
        game_token,
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
        // ── Surface « serveurs de jeu » (port `GameContainerRuntime`) ──
        //
        // Separee de la surface `DockerHost` ci-dessus : celle-la administre
        // l'hote (inventaire, purge), celle-ci pilote le cycle de vie des
        // conteneurs applicatifs. JETON DISTINCT (`guarded_game!`) — les
        // identifiants de nexus-api ne doivent pas ouvrir l'arret ou la purge
        // des conteneurs de l'hote. Meme liste blanche stricte : seize
        // operations nommees, aucun passe-plat vers l'API Docker.
        .route("/game/operational", get(game_operational))
        .route("/game/networks/{name}/ensure", post(game_ensure_network))
        .route("/game/volumes/{name}/ensure", post(game_ensure_volume))
        .route("/game/volumes/{name}/remove", post(game_remove_volume))
        .route("/game/images/pull", post(game_pull_image))
        .route("/game/images/remove", post(game_remove_image))
        .route("/game/containers", post(game_create_container))
        .route("/game/containers/managed", get(game_list_managed))
        .route("/game/containers/{id}/start", post(game_start))
        .route("/game/containers/{id}/stop", post(game_stop))
        .route("/game/containers/{id}/restart", post(game_restart))
        .route("/game/containers/{id}/remove", post(game_remove))
        .route("/game/containers/{id}/upload", post(game_upload))
        .route("/game/containers/{id}/inspect", get(game_inspect))
        .route("/game/containers/{id}/stats", get(game_stats))
        .route("/game/containers/{id}/logs", get(game_logs))
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

/// Lit un jeton obligatoire, ou refuse de demarrer.
fn required_token(var: &str) -> String {
    match std::env::var(var) {
        Ok(value) if value.trim().len() >= 16 => value,
        Ok(_) => {
            tracing::error!("{var} trop court (16 caracteres minimum)");
            std::process::exit(1);
        }
        Err(_) => {
            tracing::error!("{var} manquant");
            std::process::exit(1);
        }
    }
}

/// Verifie le jeton porte par la requete contre celui de la surface visee.
///
/// Comparaison en temps constant : une egalite `==` sur des chaines s'arrete au
/// premier octet different, ce qui laisse deviner le jeton caractere par
/// caractere via la latence. Meme precaution que `auth_middleware` cote
/// sentinel-api.
fn authorize(headers: &HeaderMap, expected_token: &str) -> Result<(), AgentError> {
    let expected = format!("Bearer {expected_token}");
    let supplied = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let ok: bool = supplied.as_bytes().ct_eq(expected.as_bytes()).unwrap_u8() == 1;

    if ok {
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

/// Surface d'administration de l'hote.
macro_rules! guarded {
    ($state:ident, $headers:ident) => {
        authorize(&$headers, &$state.host_token)?
    };
}

/// Surface serveurs de jeu. Macro DISTINCTE et non un parametre de la
/// precedente : le compilateur ne peut pas verifier qu'on a choisi le bon
/// jeton, mais deux noms differents rendent une erreur visible a la relecture.
macro_rules! guarded_game {
    ($state:ident, $headers:ident) => {
        authorize(&$headers, &$state.game_token)?
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

// ── Handlers « serveurs de jeu » ──────────────────────────────────────────

#[derive(Deserialize)]
struct GameTimeoutQuery {
    #[serde(default = "default_game_timeout")]
    timeout_secs: u32,
}
fn default_game_timeout() -> u32 {
    30
}

#[derive(Deserialize)]
struct GameLinesQuery {
    #[serde(default = "default_tail")]
    lines: u32,
}

#[derive(Deserialize)]
struct ImageRequest {
    image: String,
    #[serde(default)]
    force: bool,
}

#[derive(Deserialize)]
struct UploadRequest {
    path: String,
    content: String,
}

/// Le runtime jeu est optionnel (socket absent au boot) : on renvoie 503
/// plutot que de paniquer, l'appelant sait alors refuser proprement.
fn game(state: &AgentState) -> Result<&Arc<dyn GameContainerRuntime>, AgentError> {
    state.game.as_ref().ok_or_else(|| {
        AgentError(
            StatusCode::SERVICE_UNAVAILABLE,
            "socket Docker indisponible".to_owned(),
        )
    })
}

async fn game_operational(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<Json<bool>, AgentError> {
    guarded_game!(state, headers);
    Ok(Json(
        state
            .game
            .as_ref()
            .map(|g| g.is_operational())
            .unwrap_or(false),
    ))
}

async fn game_ensure_network(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<StatusCode, AgentError> {
    guarded_game!(state, headers);
    game(&state)?.ensure_network(&name).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn game_ensure_volume(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<StatusCode, AgentError> {
    guarded_game!(state, headers);
    game(&state)?.ensure_volume(&name).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn game_remove_volume(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<StatusCode, AgentError> {
    guarded_game!(state, headers);
    game(&state)?.remove_volume(&name).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Image en corps JSON et pas en segment d'URL : un tag Docker contient des
/// `/` et des `:` (`ghcr.io/owner/img:1.2`) qu'un `Path` decouperait.
async fn game_pull_image(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Json(body): Json<ImageRequest>,
) -> Result<StatusCode, AgentError> {
    guarded_game!(state, headers);
    game(&state)?.pull_image_if_missing(&body.image).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn game_remove_image(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Json(body): Json<ImageRequest>,
) -> Result<Json<bool>, AgentError> {
    guarded_game!(state, headers);
    Ok(Json(
        game(&state)?.remove_image(&body.image, body.force).await?,
    ))
}

async fn game_create_container(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Json(spec): Json<ContainerSpec>,
) -> Result<Json<String>, AgentError> {
    guarded_game!(state, headers);
    Ok(Json(game(&state)?.create_container(&spec).await?))
}

async fn game_list_managed(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ManagedContainer>>, AgentError> {
    guarded_game!(state, headers);
    Ok(Json(game(&state)?.list_managed_containers().await?))
}

async fn game_start(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, AgentError> {
    guarded_game!(state, headers);
    game(&state)?.start_container(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn game_stop(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<GameTimeoutQuery>,
) -> Result<StatusCode, AgentError> {
    guarded_game!(state, headers);
    game(&state)?.stop_container(&id, q.timeout_secs).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn game_restart(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<GameTimeoutQuery>,
) -> Result<StatusCode, AgentError> {
    guarded_game!(state, headers);
    game(&state)?.restart_container(&id, q.timeout_secs).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn game_remove(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, AgentError> {
    guarded_game!(state, headers);
    game(&state)?.remove_container(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn game_upload(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<UploadRequest>,
) -> Result<StatusCode, AgentError> {
    guarded_game!(state, headers);
    game(&state)?
        .upload_file_to_container(&id, &body.path, &body.content)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn game_inspect(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Option<ContainerStatus>>, AgentError> {
    guarded_game!(state, headers);
    Ok(Json(game(&state)?.inspect(&id).await?))
}

async fn game_stats(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ContainerStats>, AgentError> {
    guarded_game!(state, headers);
    Ok(Json(game(&state)?.stats(&id).await?))
}

async fn game_logs(
    Extension(state): Extension<Arc<AgentState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<GameLinesQuery>,
) -> Result<Json<Vec<String>>, AgentError> {
    guarded_game!(state, headers);
    Ok(Json(game(&state)?.logs(&id, q.lines).await?))
}
