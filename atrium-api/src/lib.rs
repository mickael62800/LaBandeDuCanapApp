//! Contrat HTTP et client DeepSeek de l'accueil IA Atrium.
//!
//! L'API ne parle pas a Discord : `atrium-bot` lui transmet un message deja
//! filtre. Cela garde les permissions Discord et les actions sensibles hors du
//! perimetre du modele.

use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use atrium_core::{
    application::{CalmingService, ServerSummaryService, WelcomeService},
    domain::{WelcomeError, WelcomePrompt},
    ports::{
        inbound::{
            GenerateCalmingReplyUseCase, GenerateServerSummaryUseCase, GenerateWelcomeReplyUseCase,
        },
        outbound::{AiProviderError, WelcomeAiGateway},
    },
};
use axum::response::IntoResponse;
use axum::{
    extract::Extension,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use platform_common_api::{rate_limit_middleware, RateLimiter};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub mod admin;
pub mod budget;
pub mod control;
pub mod guild_config;
pub mod memory;
pub mod rag;

const DEEPSEEK_URL: &str = "https://api.deepseek.com/chat/completions";

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub welcome: Arc<dyn GenerateWelcomeReplyUseCase>,
    pub calming: Arc<dyn GenerateCalmingReplyUseCase>,
    pub summary: Arc<dyn GenerateServerSummaryUseCase>,
    pub rag: Option<Arc<rag::RagService>>,
    pub budget: Option<Arc<budget::BudgetGuard>>,
    pub control: Option<Arc<control::BotControlStore>>,
    pub memory: Option<Arc<memory::ConversationMemory>>,
    /// Connexion utilisee par l'administration pour ECRIRE la config par
    /// serveur. La lecture, elle, se fait dans les stores qui en ont besoin
    /// (`budget`, `control`), au plus pres de leur usage.
    pub config_pool: Option<PgPool>,
}

#[derive(Clone)]
pub struct AppConfig {
    pub bind_addr: SocketAddr,
    pub grpc_addr: SocketAddr,
    pub rag_database_url: String,
    pub embeddings_base_url: String,
    pub embeddings_api_key: Option<String>,
    pub embeddings_model: String,
    pub user_cooldown_secs: u64,
    pub user_daily_limit: u32,
    pub global_daily_limit: u32,
    api_token: String,
    pub grpc_token: String,
    deepseek_api_key: String,
    model: String,
}

impl AppConfig {
    pub fn dummy() -> Self {
        Self {
            bind_addr: "127.0.0.1:8090".parse().unwrap(),
            grpc_addr: "127.0.0.1:8091".parse().unwrap(),
            rag_database_url: "postgres://localhost/test".into(),
            embeddings_base_url: "http://127.0.0.1:11434/v1".into(),
            embeddings_api_key: None,
            embeddings_model: "nomic-embed-text".into(),
            user_cooldown_secs: 10,
            user_daily_limit: 30,
            global_daily_limit: 500,
            api_token: "test-token".into(),
            grpc_token: "test-grpc-token".into(),
            deepseek_api_key: "test-ds-key".into(),
            model: "deepseek-v4-flash".into(),
        }
    }

    pub fn from_env() -> Result<Self, String> {
        let bind_addr = std::env::var("ATRIUM_API_BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8090".into())
            .parse()
            .map_err(|_| "ATRIUM_API_BIND_ADDR invalide".to_owned())?;
        // `std::env::var` rend `Ok("")` pour une variable DEFINIE MAIS VIDE :
        // ne tester que la presence laissait passer une chaine vide. Or un
        // jeton attendu vide est accepte par `bearer_auth::matches` des lors que
        // le client envoie l'en-tete `Authorization: Bearer ` (prefixe seul) —
        // autrement dit, `/admin/*` s'ouvrait a qui savait cela. Le compose
        // rendait le cas atteignable : `ATRIUM_API_TOKEN: ${ATRIUM_API_TOKEN:-}`.
        let required = |key: &str| {
            let value = std::env::var(key).map_err(|_| format!("variable {key} manquante"))?;
            if value.trim().is_empty() {
                return Err(format!("variable {key} vide"));
            }
            Ok(value)
        };
        // Un secret court est devinable. On ne refuse pas de demarrer (ce serait
        // bloquer une installation qui fonctionne sur un jeton court), mais le
        // silence sur ce point serait pire que l'avertissement.
        let secret = |key: &str| {
            let value = required(key)?;
            if value.len() < 16 {
                tracing::warn!(
                    "{key} fait moins de 16 caracteres — 32+ recommandes en production"
                );
            }
            Ok::<String, String>(value)
        };
        Ok(Self {
            bind_addr,
            grpc_addr: std::env::var("ATRIUM_GRPC_BIND_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8091".into())
                .parse()
                .map_err(|_| "ATRIUM_GRPC_BIND_ADDR invalide".to_owned())?,
            rag_database_url: required("ATRIUM_RAG_DATABASE_URL")?,
            embeddings_base_url: std::env::var("ATRIUM_EMBEDDINGS_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".into()),
            embeddings_api_key: std::env::var("ATRIUM_EMBEDDINGS_API_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            embeddings_model: std::env::var("ATRIUM_EMBEDDINGS_MODEL")
                .unwrap_or_else(|_| "nomic-embed-text".into()),
            user_cooldown_secs: env_u64("ATRIUM_USER_COOLDOWN_SECS", 10)?,
            user_daily_limit: env_u32("ATRIUM_USER_DAILY_LIMIT", 30)?,
            global_daily_limit: env_u32("ATRIUM_GLOBAL_DAILY_LIMIT", 500)?,
            api_token: secret("ATRIUM_API_TOKEN")?,
            grpc_token: secret("ATRIUM_GRPC_TOKEN")?,
            deepseek_api_key: secret("DEEPSEEK_API_KEY")?,
            model: std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".into()),
        })
    }
}

/// Construit l'unique `PgPool` d'Atrium, partage par tous les stores.
///
/// Auparavant chaque store (`RagService`, `BudgetGuard`, `BotControlStore`,
/// `ConversationMemory`, la config HTTP et gRPC) ouvrait son propre pool vers la
/// meme base : autant de plafonds de connexions independants, impossibles a
/// regler d'un seul endroit. Un pool unique, aux limites explicites, remplace le
/// tout — chaque store en recoit un clone (le clone partage le meme jeu de
/// connexions).
pub fn connect_pool(config: &AppConfig) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(
            std::env::var("ATRIUM_DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(10),
        )
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect_lazy(&config.rag_database_url)
}

/// Applique les migrations Atrium avant de rendre l'API disponible.
///
/// La base doit etre dediee a Atrium : SQLx partage sa table
/// `_sqlx_migrations` par base et Sentinel possede deja une migration `001`.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./migrations").run(pool).await?;
    tracing::info!("Migrations Atrium appliquees");
    Ok(())
}

/// Sert le routeur sur `listener`.
///
/// **`into_make_service_with_connect_info` n'est pas optionnel** : le rate limit
/// commun (`platform_common_api::rate_limit_middleware`) extrait
/// `ConnectInfo<SocketAddr>` pour identifier le client. Sans cette extension,
/// l'extracteur rejette et l'API repond 500 sur TOUTES ses routes, `/health`
/// compris — donc le healthcheck du conteneur echoue, atrium-api n'est jamais
/// `healthy`, et le bot comme le worker (qui l'attendent en `depends_on`) ne
/// demarrent jamais.
///
/// C'est exactement ce qui se passait : `main` appelait `axum::serve` avec le
/// routeur nu. Cette fonction existe pour qu'il n'y ait plus qu'un seul endroit
/// ou l'oublier, et pour que le test d'integration passe par le meme chemin que
/// la production.
pub async fn serve(listener: tokio::net::TcpListener, router: Router) -> std::io::Result<()> {
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
}

pub fn router(
    config: AppConfig,
    pool: PgPool,
    rag: Arc<rag::RagService>,
    budget: Arc<budget::BudgetGuard>,
    control: Arc<control::BotControlStore>,
    memory: Arc<memory::ConversationMemory>,
) -> Router {
    let state = Arc::new(AppState {
        welcome: welcome_use_case(&config),
        calming: calming_use_case(&config),
        summary: summary_use_case(&config),
        rag: Some(rag),
        budget: Some(budget),
        control: Some(control),
        memory: Some(memory),
        config_pool: Some(pool),
        config,
    });
    router_with_state(state)
}

fn env_u64(key: &str, default: u64) -> Result<u64, String> {
    std::env::var(key)
        .unwrap_or_else(|_| default.to_string())
        .parse()
        .map_err(|_| format!("variable {key} invalide"))
}

fn env_u32(key: &str, default: u32) -> Result<u32, String> {
    std::env::var(key)
        .unwrap_or_else(|_| default.to_string())
        .parse()
        .map_err(|_| format!("variable {key} invalide"))
}

pub fn router_with_state(state: Arc<AppState>) -> Router {
    let rate_limiter = RateLimiter::new(
        std::env::var("ATRIUM_API_RATE_LIMIT_PER_SEC")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(5),
    );
    let bearer =
        platform_common_api::bearer_auth::RequiredBearerToken::new(state.config.api_token.clone());
    let protected = Router::new()
        .route(
            "/admin/guilds/{guild_id}/state",
            get(admin::get_state).put(admin::set_state),
        )
        .route("/admin/guilds/{guild_id}/usage", get(admin::get_usage))
        .route(
            "/admin/guilds/{guild_id}/config",
            get(admin::get_config).put(admin::set_config),
        )
        .route(
            "/admin/guilds/{guild_id}/knowledge",
            get(admin::get_knowledge),
        )
        .route(
            "/admin/guilds/{guild_id}/jobs/summary",
            post(admin::job_generate_summary),
        )
        .route("/admin/jobs/retention", post(admin::job_retention))
        .route_layer(axum::middleware::from_fn_with_state(
            bearer,
            platform_common_api::bearer_auth::require,
        ));

    let router = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .merge(protected)
        .layer(Extension(state))
        .layer(axum::middleware::from_fn(
            platform_common_api::metrics::metrics_middleware,
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        ));

    platform_common_api::http::security_headers(router).with_state(rate_limiter)
}

/// Passerelle DeepSeek partagée par l'accueil et l'apaisement : un seul
/// adaptateur, deux cas d'usage (cf. `WelcomeAiGateway`).
fn deepseek_gateway(config: &AppConfig) -> Arc<dyn WelcomeAiGateway> {
    let client = Client::builder()
        .pool_max_idle_per_host(10)
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();
    Arc::new(DeepSeekGateway {
        client,
        api_key: config.deepseek_api_key.clone(),
        model: config.model.clone(),
    })
}

pub fn welcome_use_case(config: &AppConfig) -> Arc<dyn GenerateWelcomeReplyUseCase> {
    Arc::new(WelcomeService::new(deepseek_gateway(config)))
}

pub fn calming_use_case(config: &AppConfig) -> Arc<dyn GenerateCalmingReplyUseCase> {
    Arc::new(CalmingService::new(deepseek_gateway(config)))
}

pub fn summary_use_case(config: &AppConfig) -> Arc<dyn GenerateServerSummaryUseCase> {
    Arc::new(ServerSummaryService::new(deepseek_gateway(config)))
}

pub mod grpc;

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

/// Metriques Prometheus.
///
/// Protection optionnelle, alignee sur sentinel-api et nexus-api : si
/// `ATRIUM_METRICS_TOKEN` est defini, on exige `Authorization: Bearer <token>`.
/// Vide = ouvert, ce qui reste acceptable sur le reseau interne ou Prometheus
/// scrape, et evite d'imposer un secret de plus a une installation simple.
async fn metrics(headers: HeaderMap) -> axum::response::Response {
    // Lu une fois par processus : relire l'environnement a chaque scrape ne
    // changeait rien (l'environnement d'un processus ne bouge pas) et faisait
    // dependre un controle d'acces d'un appel systeme sur le chemin chaud.
    static CONFIGURED: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let configured =
        CONFIGURED.get_or_init(|| std::env::var("ATRIUM_METRICS_TOKEN").unwrap_or_default());
    let supplied = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    if !platform_common_api::metrics::metrics_auth_ok(Some(configured), supplied) {
        return (StatusCode::UNAUTHORIZED, "jeton metrics invalide").into_response();
    }
    platform_common_api::metrics::render_metrics()
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn merge_context(admin_context: &str, retrieved: &str) -> String {
    format!("{}\n\n{}", admin_context.trim(), retrieved.trim())
        .chars()
        .take(12_000)
        .collect()
}

#[derive(Serialize)]
struct DeepSeekRequest {
    model: String,
    messages: Vec<DeepSeekMessage>,
    max_tokens: u16,
    temperature: f32,
    thinking: DeepSeekThinking,
}
#[derive(Serialize)]
struct DeepSeekThinking {
    #[serde(rename = "type")]
    kind: &'static str,
}
#[derive(Serialize)]
struct DeepSeekMessage {
    role: &'static str,
    content: String,
}
#[derive(Deserialize)]
struct DeepSeekResponse {
    choices: Vec<DeepSeekChoice>,
}
#[derive(Deserialize)]
struct DeepSeekChoice {
    message: DeepSeekResponseMessage,
}
#[derive(Deserialize)]
struct DeepSeekResponseMessage {
    content: Option<String>,
}

struct DeepSeekGateway {
    client: Client,
    api_key: String,
    model: String,
}

#[async_trait]
impl WelcomeAiGateway for DeepSeekGateway {
    async fn generate(&self, prompt: WelcomePrompt) -> Result<String, AiProviderError> {
        let body = DeepSeekRequest {
            model: self.model.clone(),
            messages: vec![
                DeepSeekMessage {
                    role: "system",
                    content: prompt.system,
                },
                DeepSeekMessage {
                    role: "user",
                    content: prompt.user,
                },
            ],
            max_tokens: 250,
            temperature: 0.4,
            // Le chatbot d'accueil n'a pas besoin d'une longue chaine de
            // raisonnement. En mode thinking, celle-ci peut consommer tout le
            // budget et laisser `content` vide.
            thinking: DeepSeekThinking { kind: "disabled" },
        };
        let response = self
            .client
            .post(DEEPSEEK_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                tracing::warn!(%error, "Appel DeepSeek impossible");
                AiProviderError
            })?;
        let status = response.status();
        let response_body = response.text().await.map_err(|error| {
            tracing::warn!(%error, %status, "Lecture reponse DeepSeek impossible");
            AiProviderError
        })?;
        // Le corps n'est JAMAIS journalise : il porte la sortie du modele, donc
        // du contenu derive des messages des membres, et ces logs partent dans le
        // journal technique consultable en back-office. La taille et le statut
        // suffisent a diagnostiquer un refus ou une reponse malformee.
        if !status.is_success() {
            tracing::warn!(
                %status,
                octets = response_body.len(),
                "DeepSeek a refuse la requete"
            );
            return Err(AiProviderError);
        }
        let payload: DeepSeekResponse = serde_json::from_str(&response_body).map_err(|error| {
            tracing::warn!(%error, octets = response_body.len(), "Reponse DeepSeek invalide");
            AiProviderError
        })?;
        let content = payload
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .filter(|content| !content.trim().is_empty());
        if content.is_none() {
            tracing::warn!(
                octets = response_body.len(),
                "DeepSeek n'a retourne aucun contenu final"
            );
        }
        content.ok_or(AiProviderError)
    }
}

pub struct ApiError {
    status: StatusCode,
    message: &'static str,
}
impl ApiError {
    pub(crate) fn bad_request(message: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message,
        }
    }
    /// Dependance absente ou injoignable (base, store non construit).
    ///
    /// Distinct de `bad_request` : la requete est valable, c'est le service
    /// qui ne peut pas repondre. Renvoyer 400 ici enverrait l'administrateur
    /// corriger une saisie alors que le probleme est cote serveur.
    pub(crate) fn unavailable(message: &'static str) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message,
        }
    }
}
impl From<WelcomeError> for ApiError {
    fn from(_: WelcomeError) -> Self {
        Self::bad_request("requete d'accueil invalide")
    }
}
impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        platform_common_api::errors::error_response(self.status, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::merge_context;

    #[test]
    fn merge_context_trims_and_joins() {
        assert_eq!(
            merge_context("  consigne  ", "\nsavoir\n"),
            "consigne\n\nsavoir"
        );
    }

    #[test]
    fn merge_context_borne_a_12000_caracteres() {
        let long = "a".repeat(20_000);
        let merged = merge_context(&long, "ignore");
        // La borne compte des caracteres, pas des octets : on la verifie ainsi.
        assert_eq!(merged.chars().count(), 12_000);
    }

    #[test]
    fn merge_context_tronque_sans_couper_un_caractere_multioctet() {
        // 12_000 caracteres accentues : la troncature `chars().take()` ne doit
        // jamais produire d'octet UTF-8 invalide (ce que ferait une coupe sur
        // les octets). Le simple fait de construire la String le garantit.
        let accents = "é".repeat(13_000);
        let merged = merge_context(&accents, "");
        assert_eq!(merged.chars().count(), 12_000);
        assert!(merged.chars().all(|c| c == 'é'));
    }
}
