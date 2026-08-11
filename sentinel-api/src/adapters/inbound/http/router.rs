use axum::http::header;
use axum::http::HeaderValue;
use axum::http::Method;
use axum::middleware;
use axum::routing::get;
use axum::routing::post;
use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::AllowOrigin;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::MakeRequestUuid;
use tower_http::request_id::PropagateRequestIdLayer;
use tower_http::request_id::SetRequestIdLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing::Span;

use super::handlers;
use super::metrics::metrics_handler;
use super::metrics::metrics_middleware;
use super::middleware::api_logger::{api_logger_middleware, ApiLoggerState};
use super::middleware::auth::auth_middleware;
use super::middleware::rate_limit::rate_limit_middleware;
use super::middleware::rate_limit::RateLimiter;
use super::routes;
use super::state::AppState;

fn build_cors(allowed_origins: &str) -> CorsLayer {
    // Securite : `*` (AllowOrigin::any) est INCOMPATIBLE avec allow_credentials(true).
    // Le combo autoriserait n'importe quelle origine a envoyer les cookies de
    // session / le header Authorization. On desactive donc les credentials des
    // que la config est en wildcard, et on log un warning explicite.
    let wildcard = allowed_origins == "*";
    if wildcard {
        tracing::warn!(
            "ALLOWED_ORIGINS=* : CORS en mode permissif SANS credentials. \
             Pour autoriser les cookies de session, lister les origines exactes."
        );
    }
    let allow_origin = if wildcard {
        AllowOrigin::any()
    } else if allowed_origins.is_empty() {
        // Default securise : uniquement les origines Tauri + localhost dev
        tracing::info!("ALLOWED_ORIGINS non configure — utilisation des origines par defaut (Tauri + localhost)");
        AllowOrigin::list([
            "https://tauri.localhost".parse::<HeaderValue>().unwrap(),
            "http://tauri.localhost".parse::<HeaderValue>().unwrap(),
            "http://localhost:1420".parse::<HeaderValue>().unwrap(),
            "http://localhost:3000".parse::<HeaderValue>().unwrap(),
        ])
    } else {
        let origins: Vec<HeaderValue> = allowed_origins
            .split(',')
            .filter_map(|o| o.trim().parse().ok())
            .collect();
        AllowOrigin::list(origins)
    };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::HeaderName::from_static("x-request-id"),
            header::HeaderName::from_static("x-discord-token"),
            header::HeaderName::from_static("x-api-key"),
        ])
        // Cookies de session (refresh token) : requis pour fetch credentials.
        // En prod le front est same-origin (reverse proxy) donc CORS ne joue
        // pas ; en dev cross-origin, ALLOWED_ORIGINS doit lister l'origine exacte
        // (pas `*`) pour que les cookies soient acceptes par le navigateur.
        // credentials desactive en wildcard (cf. supra) — sinon faille CORS.
        .allow_credentials(!wildcard)
        .max_age(std::time::Duration::from_secs(3600))
}

/// Compose toutes les routes protegees par auth (hors endpoints lourds).
fn protected_domain_routes() -> Router<AppState> {
    Router::new()
        // Bot-facing routes (scoring, rules, infractions) — sans /analyze (deplace dans heavy)
        .merge(routes::bot::routes())
        // App-facing routes (nested by domain)
        .merge(routes::ticket::routes())
        .merge(routes::idea::routes())
        .merge(routes::security::routes())
        .merge(routes::automod::routes())
        .merge(routes::moderation::routes())
        .merge(routes::stats::routes())
        // Dashboard & config routes + charts
        .merge(routes::dashboard::routes())
        // Audit logs + watched users + discord roles
        .merge(routes::audit::routes())
        // Bot persistence (fire-and-forget)
        .merge(routes::bot_persistence::routes())
        // Members + guild direct API
        .merge(routes::guild_backup::routes())
        .merge(routes::guilds::routes())
        .merge(routes::members::routes())
        .merge(routes::role_panels::routes())
        .merge(routes::voice_channels::routes())
        .merge(routes::progression::routes())
        .merge(routes::guild_structure::routes())
        .merge(routes::community::routes())
        // Système + jobs async + RBAC + welcome
        .merge(routes::system::routes())
}

/// Routes accessibles SANS authentification.
///
/// Source unique, partagee par `build` et `build_for_test`. Ces deux routeurs
/// sont independants, et declarer une route dans un seul donnait des tests
/// verts sur une API qui repondait 404 en production — c'est exactement ce qui
/// est arrive a tout `/api/public/*`. Une fonction commune rend la
/// desynchronisation impossible.
///
/// Ce qui entre ici n'exige aucune identite, donc ne doit exposer aucune
/// donnee personnelle. Chaque handler ecrit son DTO champ par champ et force
/// son filtre restrictif : les parametres du back-office (`?all=1`) ne peuvent
/// pas y faire remonter brouillons ni archives.
///
/// `/metrics` n'y figure pas : il porte son propre jeton et reste declare
/// cote production uniquement.
fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::system::health::health))
        // Le flux OAuth Discord (`/auth/discord/*`, `/auth/refresh`,
        // `/auth/logout`) a ete EXTRAIT dans `auth-api`. Ces chemins existent
        // toujours pour le navigateur : nginx les route vers l'identite, pas
        // ici. Ne pas les recreer — deux implementations du meme flux, dont une
        // seule est cablee, est une invitation a debugger la mauvaise.
        // ── Site communautaire ──
        .route(
            "/api/public/guilds/{guild_id}",
            get(handlers::system::public_site::public_guild),
        )
        // Planning : uniquement les evenements publies ET publics, via un DTO
        // distinct (cf. handlers::community::events).
        .route(
            "/api/public/events/{guild_id}",
            get(handlers::community::events::public_events),
        )
        .route(
            "/api/public/lfg/{guild_id}",
            get(handlers::community::lfg::public_lfg),
        )
        .route(
            "/api/public/polls/{guild_id}",
            get(handlers::community::polls::public_polls),
        )
        .route(
            "/api/public/spotlight/{guild_id}",
            get(handlers::community::spotlight::public_spotlight),
        )
        .route(
            "/api/public/news/{guild_id}",
            get(handlers::community::news::public_news),
        )
        .route(
            "/api/public/pulse/{guild_id}",
            get(handlers::community::pulse::public_pulse),
        )
        // Presence en direct : le bot ne publie que les salons visibles par
        // @everyone, l'API n'a aucune vue sur les permissions Discord.
        .route(
            "/api/public/presence/{guild_id}",
            get(handlers::community::presence::public_presence),
        )
}

/// Construit le router sans rate limiter ni ConnectInfo — pour les tests d'integration.
pub fn build_for_test(state: AppState) -> Router {
    let protected = Router::new()
        // Endpoints lourds (sans rate limit en test)
        .route("/analyze", post(handlers::ai::analyze::analyze))
        .route(
            "/analyze/image",
            post(handlers::ai::analyze_image::analyze_image),
        )
        .merge(routes::analytics::routes())
        // Routes standard
        .merge(protected_domain_routes())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let public = public_routes();

    Router::new()
        .merge(protected)
        .merge(public)
        .with_state(state)
}

pub fn build(
    state: AppState,
    max_body_size: usize,
    rate_limit_per_sec: u64,
    allowed_origins: &str,
) -> Router {
    let limiter = RateLimiter::new(rate_limit_per_sec);

    // Limiter strict pour les endpoints lourds (inference IA, analytics)
    // Par defaut : 5 req/s (burst 50) vs standard qui est typiquement 50-100 req/s
    let heavy_rate: u64 = std::env::var("HEAVY_RATE_LIMIT_PER_SEC")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let heavy_limiter = RateLimiter::new(heavy_rate);

    // Spawn cleanup tasks (purge stale IP buckets every 60s)
    let limiter_cleanup = limiter.clone();
    let heavy_cleanup = heavy_limiter.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            limiter_cleanup.cleanup().await;
            heavy_cleanup.cleanup().await;
        }
    });

    // Routes lourdes avec rate limit strict (inference IA + analytics)
    let heavy_routes = Router::new()
        .route("/analyze", post(handlers::ai::analyze::analyze))
        .route(
            "/analyze/image",
            post(handlers::ai::analyze_image::analyze_image),
        )
        .merge(routes::analytics::routes())
        .route_layer(middleware::from_fn_with_state(
            heavy_limiter,
            rate_limit_middleware,
        ));

    // Routes protegees par auth + rate limit standard
    let protected = Router::new()
        // Routes lourdes (limiter strict)
        .merge(heavy_routes)
        // Toutes les routes de domaine protegees
        .merge(protected_domain_routes())
        // Gate d'acces unique du back-office : seuls les Discord user IDs
        // listes dans SUPERADMIN_USER_IDS (.env) passent. Les services
        // internes (bot/workers, Bearer API_KEY) restent autorises. Remplace
        // l'ancienne pile RBAC multi-roles (rbac + whitelist + guild_auth +
        // global_rbac). Doit tourner apres auth_middleware, qui pose le
        // marqueur AuthKind.
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::adapters::inbound::http::middleware::superadmin::superadmin_middleware,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .route_layer(middleware::from_fn_with_state(
            limiter,
            rate_limit_middleware,
        ));

    // Routes publiques (health + métriques Prometheus pour scraping)
    //
    // `/metrics` : ouvert par defaut (Prometheus scrape sans auth sur le reseau
    // interne ; le port API est bind 127.0.0.1 et non proxifie par nginx).
    // Pour durcir : definir METRICS_TOKEN cote API + configurer l'`authorization`
    // du job Prometheus avec le meme token (cf. metrics_handler).
    // `/metrics` porte son propre jeton et n'a de sens qu'en production.
    let public = public_routes().route("/metrics", get(metrics_handler));

    // Helper : true pour les endpoints bruyants (heartbeat des bots toutes
    // les 1-3s, /health du frontend toutes les 90s). On veut les voir en
    // DEBUG pour ne pas polluer les logs INFO.
    fn is_low_verbosity_path(p: &str) -> bool {
        p.contains("/heartbeat") || p == "/health"
    }

    // TraceLayer configure pour inclure le request_id dans chaque span
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|request: &axum::http::Request<_>| {
            let request_id = request
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("-");
            let path = request.uri().path();
            let low = is_low_verbosity_path(path);

            if low {
                tracing::debug_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    request_id = %request_id,
                    low_verbosity = true,
                )
            } else {
                tracing::info_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    request_id = %request_id,
                )
            }
        })
        .on_response(
            |response: &axum::http::Response<_>, latency: std::time::Duration, span: &Span| {
                let status = response.status().as_u16();
                let latency_ms = latency.as_millis() as u64;
                // Si la span est marquee low_verbosity (heartbeat/health), on emet en DEBUG.
                // Sinon INFO. tracing-subscriber filtre selon RUST_LOG.
                if span.field("low_verbosity").is_some() {
                    tracing::debug!(status = status, latency_ms = latency_ms, "response");
                } else {
                    tracing::info!(status = status, latency_ms = latency_ms, "response");
                }
            },
        );

    let logger_state = ApiLoggerState::from_app(&state);

    Router::new()
        .merge(protected)
        .merge(public)
        // Verrou mono-serveur, applique a TOUT le routeur — protege et public
        // confondus. Un controle par handler aurait laisse passer la premiere
        // route ajoutee sans y penser.
        //
        // Pose ici, sous le logger : une requete refusee reste tracee, ce qui
        // permet de voir qu'une autre guilde a tente d'entrer.
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::adapters::inbound::http::middleware::single_guild::single_guild_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            logger_state,
            api_logger_middleware,
        ))
        // Métriques Prometheus : enregistre count + latency par (route, method, status).
        // Doit s'appliquer APRÈS le matching de route pour récupérer le `MatchedPath`.
        .layer(middleware::from_fn(metrics_middleware))
        // Phase 1 — Quick wins : compression HTTP (zstd préféré, gzip fallback).
        // S'applique sur toutes les réponses dont le client envoie un Accept-Encoding
        // compatible. Gain typique : -60 % de bande passante sur les payloads JSON
        // (la plupart de nos endpoints retournent du JSON très répétitif). Le coût
        // CPU côté serveur est négligeable à zstd niveau 3.
        .layer(CompressionLayer::new().zstd(true).gzip(true))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(RequestBodyLimitLayer::new(max_body_size))
        .layer(trace_layer)
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        // Security headers
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        // Content-Security-Policy strict : l'API ne sert que du JSON, aucune
        // execution de script / chargement de ressource n'est legitime sur ce
        // domaine. Bloque tout XSS reflechi residuel.
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'",
            ),
        ))
        .layer(build_cors(allowed_origins))
        .with_state(state)
}
