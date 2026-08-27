//! Couche HTTP axum : router, pile de middlewares, auth Bearer, handlers.
//!
//! La pile est alignee sur `sentinel-api` (meme ordre, memes garanties). Nexus
//! expose sa propre surface reseau et pilote Docker : il ne peut pas etre moins
//! protege que l'API de moderation.
//!
//! Ordre de traversee d'une requete :
//!
//! ```text
//! CORS → en-tetes de securite → trace → limite de corps → request-id
//!      → compression → metriques → verrou mono-serveur
//!      → [routes /api] rate limit → Bearer → (rate limit strict si lifecycle)
//!      → handler
//! ```

pub mod dto;
pub mod handlers;
pub mod metrics;

use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{delete, get, patch, post, put};
use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::MakeRequestUuid;
use tower_http::request_id::PropagateRequestIdLayer;
use tower_http::request_id::SetRequestIdLayer;
use tower_http::trace::TraceLayer;

use crate::nexus::bootstrap::AppState;
use crate::shared::rate_limit_middleware;
use crate::shared::RateLimiter;

/// Reglages de la couche HTTP, lus une fois au demarrage.
#[derive(Clone, Debug)]
pub struct HttpConfig {
    /// Taille maximale du corps d'une requete, en octets.
    pub max_body_size: usize,
    /// Debit soutenu autorise par IP sur les routes de lecture.
    pub rate_limit_per_sec: u64,
    /// Debit soutenu par IP sur les routes qui pilotent des conteneurs.
    pub heavy_rate_limit_per_sec: u64,
    /// Origines CORS autorisees : `*`, liste separee par virgules, ou vide.
    pub allowed_origins: String,
}

impl HttpConfig {
    /// Valeurs par defaut prudentes, surchargeables par variables d'env.
    ///
    /// Le debit strict est bas (2 req/s) volontairement : derriere ces routes
    /// il y a un `docker run`. Un humain qui clique n'atteint jamais cette
    /// limite ; une boucle, si.
    pub fn from_env() -> Self {
        fn var_parse<T: std::str::FromStr>(nom: &str, defaut: T) -> T {
            std::env::var(nom)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaut)
        }

        Self {
            max_body_size: var_parse("NEXUS_MAX_BODY_SIZE", 10 * 1024 * 1024),
            rate_limit_per_sec: var_parse("NEXUS_RATE_LIMIT_PER_SEC", 50),
            heavy_rate_limit_per_sec: var_parse("NEXUS_HEAVY_RATE_LIMIT_PER_SEC", 2),
            allowed_origins: std::env::var("NEXUS_ALLOWED_ORIGINS")
                .or_else(|_| std::env::var("ALLOWED_ORIGINS"))
                .unwrap_or_default(),
        }
    }
}

/// Origines CORS acceptees quand la configuration est vide (developpement).
const ORIGINES_DEV: &[&str] = &[
    "http://localhost:1420",
    "http://localhost:3000",
    "http://localhost:5173",
];

/// Routes qui declenchent une operation Docker (creation, cycle de vie, RCON).
///
/// Isolees pour porter un rate limit strict : une requete ici peut lancer un
/// conteneur, allouer un port et reserver plusieurs Go de RAM. Elles restent
/// protegees par le Bearer et le verrou mono-serveur comme les autres.
fn container_lifecycle_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/games/{guild_id}/servers",
            post(handlers::game::servers::create_server),
        )
        .route(
            "/api/games/servers/{server_id}/start",
            post(handlers::game::servers::start_server),
        )
        .route(
            "/api/games/servers/{server_id}/stop",
            post(handlers::game::servers::stop_server),
        )
        .route(
            "/api/games/servers/{server_id}/restart",
            post(handlers::game::servers::restart_server),
        )
        .route(
            "/api/games/servers/{server_id}/reveal-ip",
            post(handlers::game::servers::reveal_ip),
        )
        .route(
            "/api/games/servers/{server_id}/reveal-ip/request",
            post(handlers::game::servers::request_reveal_ip),
        )
        .route(
            "/api/games/servers/{server_id}/schedule",
            post(handlers::game::servers::schedule_server),
        )
        .route(
            "/api/games/servers/{server_id}/reveal-schedule",
            post(handlers::game::servers::set_reveal_schedule),
        )
        .route(
            "/api/games/servers/{server_id}/command",
            post(handlers::game::servers::execute_rcon),
        )
        // Catalogue d'administration : le navigateur envoie une CLE et des
        // parametres, jamais une commande. Meme rate limit strict que la
        // console libre — derriere, c'est le meme RCON.
        .route(
            "/api/games/servers/{server_id}/commands/{command_key}",
            post(handlers::game::servers::run_catalog_command),
        )
        .route(
            "/api/games/servers/{server_id}/players/online",
            get(handlers::game::servers::list_online_players),
        )
        .route(
            "/api/games/servers/{server_id}/stream-logs",
            get(handlers::game::servers::stream_logs_sse),
        )
        .route(
            "/api/games/servers/{server_id}/stream-stats",
            get(handlers::game::servers::stream_stats_sse),
        )
        .route(
            "/api/games/servers/{server_id}",
            delete(handlers::game::servers::delete_server),
        )
}

/// Construit le router complet.
pub fn build_router(state: AppState) -> Router {
    build_router_with(state, HttpConfig::from_env())
}

/// Variante explicite, utilisable en test sans toucher aux variables d'env.
pub fn build_router_with(state: AppState, config: HttpConfig) -> Router {
    let limiter = RateLimiter::new(config.rate_limit_per_sec);
    let heavy_limiter = RateLimiter::new(config.heavy_rate_limit_per_sec);
    // Bucket distinct pour la vitrine publique : elle n'est pas authentifiee,
    // son trafic ne doit donc ni consommer ni etre gene par le quota des
    // appels internes.
    let public_limiter = RateLimiter::new(config.rate_limit_per_sec);
    let bearer = crate::shared::bearer_auth::RequiredBearerToken::new(state.api_key.clone())
        .with_scheduler(std::env::var("NEXUS_SCHEDULER_TOKEN").unwrap_or_default());

    let heavy = container_lifecycle_routes().route_layer(middleware::from_fn_with_state(
        heavy_limiter,
        rate_limit_middleware,
    ));

    let api = Router::new()
        // ── Hauts faits ──
        .route(
            "/api/achievements/definitions",
            get(handlers::achievements::list_definitions),
        )
        .route(
            "/api/achievements/definitions/{id}",
            patch(handlers::achievements::update_definition),
        )
        .route(
            "/api/achievements/{guild_id}/members/{user_id}",
            get(handlers::achievements::member_progress),
        )
        .route(
            "/api/achievements/{guild_id}/links/{user_id}/{game}",
            get(handlers::achievements::get_link)
                .put(handlers::achievements::put_link)
                .delete(handlers::achievements::delete_link),
        )
        .route(
            "/api/achievements/{guild_id}/grant",
            post(handlers::achievements::grant),
        )
        .route(
            "/api/achievements/{guild_id}/game-events",
            post(handlers::achievements::game_event),
        )
        .route(
            "/api/grand-salon/{guild_id}/membership/{user_id}",
            get(handlers::grand_salon::membership),
        )
        .route(
            "/api/grand-salon/{guild_id}/habitues/{user_id}",
            get(handlers::grand_salon::profile).post(handlers::grand_salon::join),
        )
        .route(
            "/api/grand-salon/{guild_id}/habitues/{user_id}/daily",
            post(handlers::grand_salon::daily),
        )
        .route(
            "/api/grand-salon/{guild_id}/motions",
            get(handlers::grand_salon::motions).post(handlers::grand_salon::propose),
        )
        .route(
            "/api/grand-salon/{guild_id}/motions/{motion_id}/vote",
            post(handlers::grand_salon::vote),
        )
        .route(
            "/api/grand-salon/{guild_id}/gazette",
            get(handlers::grand_salon::gazette),
        )
        .route(
            "/api/grand-salon/{guild_id}/cercles",
            get(handlers::grand_salon::cercles).post(handlers::grand_salon::create_cercle),
        )
        .route(
            "/api/grand-salon/{guild_id}/dossiers/{user_id}",
            get(handlers::grand_salon::dossiers),
        )
        .route(
            "/api/grand-salon/{guild_id}/dossiers",
            post(handlers::grand_salon::investigate),
        )
        .route(
            "/api/grand-salon/{guild_id}/dossiers/{dossier_id}/reveal",
            post(handlers::grand_salon::reveal),
        )
        .route(
            "/api/grand-salon/internal/jobs/close-motions",
            post(handlers::grand_salon::close_due),
        )
        .route(
            "/api/wheel/{guild_id}/{user_id}/spin",
            post(handlers::wheel::spin),
        )
        .route(
            "/api/wheel/{guild_id}/{user_id}/status",
            get(handlers::wheel::status),
        )
        .route(
            "/api/wallet/{guild_id}/transfer",
            post(handlers::wallet::transfer),
        )
        .route(
            "/api/wallet/{guild_id}/leaderboard",
            get(handlers::wallet::leaderboard),
        )
        .route(
            "/api/wallet/{guild_id}/{user_id}",
            get(handlers::wallet::get),
        )
        .route(
            "/api/wallet/{guild_id}/{user_id}/history",
            get(handlers::wallet::history),
        )
        // ── Game Portal : catalogue jeux et panneaux Discord ──
        .route(
            "/api/games/{guild_id}",
            get(handlers::casino::games::list_games),
        )
        .route(
            "/api/games/{guild_id}/detect-mentions",
            post(handlers::casino::games::detect_mentions),
        )
        .route("/api/games", post(handlers::casino::games::create_game))
        .route(
            "/api/games/{guild_id}/{game_id}/role",
            put(handlers::casino::games::set_game_role),
        )
        .route(
            "/api/bots/definitions",
            get(handlers::bot_config::get_definitions),
        )
        .route(
            "/api/config/{guild_id}/{bot_name}",
            get(handlers::bot_config::get_config).put(handlers::bot_config::set_config),
        )
        .route(
            "/api/wheel/{guild_id}/cases",
            get(handlers::wheel::list_cases).put(handlers::wheel::replace_cases),
        )
        .route(
            "/api/coussin/{guild_id}/{user_id}/profile",
            get(handlers::coussin::profile),
        )
        .route(
            "/api/coussin/{guild_id}/{user_id}/class",
            post(handlers::coussin::choose_class),
        )
        .route(
            "/api/coussin/{guild_id}/{user_id}/train",
            post(handlers::coussin::train),
        )
        .route(
            "/api/coussin/{guild_id}/{user_id}/inventory",
            get(handlers::coussin::inventory),
        )
        .route(
            "/api/coussin/{guild_id}/{user_id}/shop",
            post(handlers::coussin::buy_item),
        )
        .route(
            "/api/coussin/{guild_id}/{user_id}/insurance",
            get(handlers::coussin::insurance).post(handlers::coussin::buy_insurance),
        )
        .route(
            "/api/coussin/{guild_id}/{user_id}/steal",
            post(handlers::coussin::steal),
        )
        // Fenetre de defense de la fouille. Le segment litteral `steals` prime
        // sur `{guild_id}` : ces routes ne peuvent pas etre confondues.
        .route(
            "/api/coussin/steals/{attempt_id}/message",
            put(handlers::coussin::attach_steal_message),
        )
        .route(
            "/api/coussin/steals/{attempt_id}/defend/{victim_id}",
            post(handlers::coussin::defend_steal),
        )
        .route(
            "/api/coussin/{guild_id}/{user_id}/prime",
            post(handlers::coussin::place_prime),
        )
        .route(
            "/api/coussin/{guild_id}/{user_id}/bets",
            post(handlers::coussin::place_bet),
        )
        .route(
            "/api/coussin/{guild_id}/classement",
            get(handlers::coussin::ranking),
        )
        .route(
            "/api/coussin/{guild_id}/{user_id}/combats",
            get(handlers::coussin::combat_history),
        )
        .route(
            "/api/coussin/{guild_id}/combats",
            post(handlers::coussin::challenge),
        )
        .route(
            "/api/coussin/combats/{id}/accept",
            post(handlers::coussin::accept),
        )
        .route(
            "/api/coussin/combats/{id}/refuse",
            post(handlers::coussin::refuse),
        )
        .route(
            "/api/coussin/combats/{id}/resolve",
            post(handlers::coussin::resolve),
        )
        .route(
            "/api/games/{guild_id}/by-category",
            get(handlers::casino::games::list_games_by_category),
        )
        .route(
            "/api/games/{guild_id}/{game_id}",
            put(handlers::casino::games::update_game).delete(handlers::casino::games::delete_game),
        )
        .route(
            "/api/games/{guild_id}/{game_id}/role",
            patch(handlers::casino::games::set_role_id),
        )
        .route(
            "/api/games/{guild_id}/by-name/{game_name}",
            get(handlers::casino::games::get_game_by_name),
        )
        .route(
            "/api/games/{guild_id}/panels",
            get(handlers::casino::games::list_panels).post(handlers::casino::games::save_panel),
        )
        .route(
            "/api/games/{guild_id}/panels/{message_id}",
            get(handlers::casino::games::find_panel_by_message),
        )
        .route(
            "/api/games/{guild_id}/panel/deploy",
            post(handlers::casino::games::deploy_panel),
        )
        // ── Consolidation base <-> Discord ──
        // Le segment litteral `sync` prime sur `{game_id}` : ces routes ne
        // peuvent pas etre confondues avec un identifiant de jeu.
        .route(
            "/api/games/{guild_id}/sync",
            get(handlers::casino::game_sync::get_report),
        )
        .route(
            "/api/games/{guild_id}/sync/check",
            post(handlers::casino::game_sync::request_check),
        )
        .route(
            "/api/games/{guild_id}/sync/inventory",
            put(handlers::casino::game_sync::put_inventory),
        )
        .route(
            "/api/games/{guild_id}/sync/roles/{role_id}",
            delete(handlers::casino::game_sync::role_vanished),
        )
        .route(
            "/api/games/{guild_id}/sync/resolve",
            post(handlers::casino::game_sync::resolve),
        )
        .route(
            "/api/games/{guild_id}/upload-emoji",
            post(handlers::casino::games::upload_emoji),
        )
        // ── Game Portal : serveurs, templates et inscriptions ──
        // Le POST (creation) vit dans `container_lifecycle_routes` : rate limit
        // strict. Seule la lecture reste ici.
        .route(
            "/api/games/{guild_id}/servers",
            get(handlers::game::servers::list_servers),
        )
        .route(
            "/api/games/servers/{server_id}/commands",
            get(handlers::game::servers::list_commands),
        )
        // Ressources : effet differe a la reconstruction du conteneur, mais
        // l'ecriture elle-meme ne pilote aucun conteneur.
        .route(
            "/api/games/servers/{server_id}/resources",
            put(handlers::game::servers::update_resources),
        )
        .route(
            "/api/games/servers/{server_id}/channel-names",
            put(handlers::game::servers::update_channel_names),
        )
        .route(
            "/api/games/servers/{server_id}/rules",
            put(handlers::game::servers::update_rules),
        )
        .route(
            "/api/games/servers/{server_id}/backup",
            post(handlers::game::servers::backup_now),
        )
        .route(
            "/api/games/servers/{server_id}/backups",
            get(handlers::game::servers::list_backups),
        )
        .route(
            "/api/games/servers/{server_id}/announcement",
            get(handlers::game::servers::get_session_announcement),
        )
        .route(
            "/api/games/servers/{server_id}/announcement/posted",
            post(handlers::game::servers::mark_session_announcement_posted),
        )
        .route(
            "/api/games/servers/{server_id}/schedule-ranges",
            get(handlers::game::servers::get_schedule_ranges)
                .put(handlers::game::servers::save_schedule_ranges),
        )
        .route(
            "/api/games/servers/{server_id}/alerts",
            get(handlers::game::servers::get_alert_settings)
                .put(handlers::game::servers::save_alert_settings)
                .delete(handlers::game::servers::delete_alert_settings),
        )
        .route(
            "/api/games/{guild_id}/templates",
            get(handlers::game::templates::list_templates_for_guild),
        )
        .route(
            "/api/games/{guild_id}/template-settings",
            get(handlers::game::session_events::list_template_settings),
        )
        .route(
            "/api/games/{guild_id}/template-settings/{slug}",
            put(handlers::game::session_events::set_template_role),
        )
        .route(
            "/api/games/templates/{id}",
            get(handlers::game::templates::get_template),
        )
        // DELETE, start, stop, restart et command sont dans
        // `container_lifecycle_routes` (rate limit strict).
        .route(
            "/api/games/servers/{server_id}",
            get(handlers::game::servers::get_server),
        )
        .route(
            "/api/games/servers/{server_id}/logs",
            get(handlers::game::servers::get_logs),
        )
        .route(
            "/api/games/servers/{server_id}/stats",
            get(handlers::game::servers::get_stats),
        )
        .route(
            "/api/games/servers/{server_id}/config",
            put(handlers::game::servers::update_config),
        )
        .route(
            "/api/games/servers/{server_id}/sessions",
            get(handlers::game::sessions::list_sessions),
        )
        .route(
            "/api/games/servers/{server_id}/perf-history",
            get(handlers::game::perf_history::get_perf_history),
        )
        .route(
            "/api/games/servers/{server_id}/registrations",
            get(handlers::game::session_events::list_registrations)
                .post(handlers::game::session_events::register_player),
        )
        .route(
            "/api/games/servers/{server_id}/registrations/{user_id}",
            delete(handlers::game::session_events::unregister_player),
        )
        .route(
            "/api/games/servers/{server_id}/session-channels",
            patch(handlers::game::session_events::set_session_channels),
        )
        // Endpoints de travail : uniquement appeles par platform-scheduler.
        .route(
            "/api/games/internal/jobs/health-check",
            post(handlers::game::jobs::job_health_check),
        )
        .route(
            "/api/games/internal/jobs/idle-shutdown",
            post(handlers::game::jobs::job_idle_shutdown),
        )
        .route(
            "/api/games/internal/jobs/reconcile",
            post(handlers::game::jobs::job_reconcile),
        )
        .route(
            "/api/games/internal/jobs/image-cleanup",
            post(handlers::game::jobs::job_image_cleanup),
        )
        .route(
            "/api/games/internal/jobs/reveal-ip",
            post(handlers::game::jobs::job_reveal_ip),
        )
        .route(
            "/api/games/internal/jobs/purge-history",
            post(handlers::game::jobs::job_purge_history),
        )
        .route(
            "/api/games/internal/jobs/daily-ping",
            post(handlers::game::jobs::job_daily_ping),
        )
        .route(
            "/api/games/internal/jobs/auto-start",
            post(handlers::game::jobs::job_auto_start),
        )
        .route(
            "/api/games/internal/jobs/mention-sync",
            post(handlers::game::jobs::job_mention_sync),
        )
        .route(
            "/api/games/internal/jobs/coussin-expire-combats",
            post(handlers::game::jobs::job_coussin_expire_combats),
        )
        .route(
            "/api/games/internal/jobs/coussin-expire-steals",
            post(handlers::game::jobs::job_coussin_expire_steals),
        )
        .route(
            "/api/games/internal/jobs/game-alerts",
            post(handlers::game::jobs::job_game_alerts),
        )
        .route(
            "/api/games/internal/jobs/game-schedules",
            post(handlers::game::jobs::job_game_schedules),
        )
        .route(
            "/api/games/internal/jobs/session-announcements",
            post(handlers::game::jobs::job_session_announcements),
        )
        // Les routes de cycle de vie des conteneurs rejoignent le groupe
        // protege : elles heritent du Bearer et du verrou mono-serveur, et
        // portent en plus leur rate limit strict.
        .merge(heavy)
        .layer(middleware::from_fn_with_state(
            bearer,
            crate::shared::bearer_auth::require,
        ))
        // Pose APRES l'auth donc traverse AVANT elle : une inondation de
        // requetes non authentifiees doit etre coupee sans consulter l'etat.
        .layer(middleware::from_fn_with_state(
            limiter,
            rate_limit_middleware,
        ));

    // Vitrine publique : montee HORS du groupe protege par le Bearer, comme
    // /health. Le DTO est ecrit champ par champ (cf. public_servers.rs).
    //
    // Pas de Bearer ne veut pas dire pas de limite : c'est la seule route que
    // n'importe qui sur Internet peut appeler, donc elle porte son propre rate
    // limit, plus genereux que le strict mais borne.
    let public = Router::new()
        .route(
            "/api/public/games/{guild_id}/servers",
            get(handlers::game::public_servers::public_servers),
        )
        .route_layer(middleware::from_fn_with_state(
            public_limiter,
            rate_limit_middleware,
        ));

    let routes = Router::new()
        .route("/health", get(|| async { "ok" }))
        // Expose les metriques pour Prometheus. Protegeable par
        // NEXUS_METRICS_TOKEN ; hors du groupe Bearer car le scraper porte son
        // propre jeton (ou aucun, sur le reseau interne).
        .route("/metrics", get(metrics::metrics_handler))
        .merge(public)
        .merge(api)
        .layer(middleware::from_fn_with_state(
            state.job_pool.clone(),
            crate::shared::job_lock::middleware,
        ))
        // Verrou mono-serveur applique a TOUT le routeur, public compris.
        // Nexus expose sa propre surface : le verrou de sentinel-api, qui
        // vit dans un autre processus, ne le protege pas.
        .layer(middleware::from_fn_with_state(state.clone(), single_guild))
        // Metriques : pose ici pour que `MatchedPath` soit deja resolu, sinon
        // toutes les series retomberaient sur le label `unknown`.
        .layer(middleware::from_fn(metrics::metrics_middleware))
        // Compression des reponses JSON (zstd prefere, gzip en repli). Les
        // listes de serveurs et de templates sont tres repetitives.
        .layer(CompressionLayer::new().zstd(true).gzip(true))
        .layer(PropagateRequestIdLayer::x_request_id())
        // Borne la memoire consommee par requete : sans ca, un POST de 2 Go
        // est bufferise avant meme d'atteindre le handler.
        .layer(RequestBodyLimitLayer::new(config.max_body_size))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
                let request_id = request
                    .headers()
                    .get("x-request-id")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("-");
                // /health est appele en boucle par Docker : en DEBUG pour ne
                // pas noyer les logs utiles.
                if request.uri().path() == "/health" {
                    tracing::debug_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri(),
                        request_id = %request_id,
                    )
                } else {
                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri(),
                        request_id = %request_id,
                    )
                }
            }),
        )
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid));

    crate::shared::http::security_headers(routes)
        .layer(crate::shared::http::build_cors(
            &config.allowed_origins,
            ORIGINES_DEV,
            &[],
        ))
        .with_state(state)
}

/// Refuse toute requete portant un `guild_id` autre que celui configure.
///
/// L'application ne sert qu'un serveur Discord. La colonne `guild_id` reste
/// dans le modele de donnees — la retirer serait un refactor massif pour
/// aucun gain — mais la surface HTTP n'accepte qu'une valeur.
///
/// Les requetes sans identifiant de serveur (sante, routes globales) passent,
/// de meme que TOUT si la variable n'est pas configuree : une installation
/// qui ne l'a pas encore renseignee ne doit pas tomber en panne.
async fn single_guild(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let Some(attendu) = state.guild_id.clone() else {
        return Ok(next.run(req).await);
    };

    // Toutes les routes concernees portent le `guild_id` dans leur chemin.
    // On cherche le premier segment qui ressemble a un identifiant Discord :
    // ici un faux positif provoque un REFUS, d'ou la fenetre stricte de 17 a
    // 20 chiffres, qui ecarte les uuid et les petits entiers.
    let trouve = req
        .uri()
        .path()
        .split('/')
        .take(5) // guild_id se trouve max au 4eme segment
        .find(|seg| (17..=20).contains(&seg.len()) && seg.chars().all(|c| c.is_ascii_digit()))
        .map(str::to_string);

    if let Some(gid) = trouve {
        if gid != attendu {
            tracing::warn!(
                guild_id = %gid,
                attendu = %attendu,
                "mono-serveur : requete refusee pour une autre guilde"
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Ok(next.run(req).await)
}
// Surface HTTP de NEXUS. Les routes sont réparties par capacité : games,
// wallet, wheel, coussin, grand_salon et bot_config. Les routes privées sont
// protégées par Bearer et les opérations runtime ont un rate-limit séparé.
