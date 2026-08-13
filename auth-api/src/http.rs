//! Surface HTTP de l'identité.
//!
//! Trois familles de routes, et la distinction compte pour la sécurité :
//!
//! - **Publiques** (`/auth/discord/*`) — traversées par un navigateur au fil du
//!   flux OAuth. Pas de jeton de service : c'est un utilisateur anonyme qui
//!   arrive, c'est tout l'objet du login.
//! - **Session** (`/auth/refresh`, `/auth/logout`) — authentifiées par le
//!   cookie `ds_session`, donc par l'utilisateur lui-même.
//! - **Service** (`/access`, `/security/*`) — réservées aux appelants internes
//!   porteurs de `AUTH_API_TOKEN` : nginx pour l'`auth_request`, sentinel-api
//!   et ops-api comme consommateurs.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Extension, Query},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use auth_core::domain::entities::session::SESSION_MAX_AGE_SECS;
use auth_core::domain::errors::DomainError;
use auth_core::ports::inbound::manage_session::{LoginContext, ManageSessionUseCase};
use auth_core::ports::inbound::resolve_access::ResolveAccessUseCase;

use crate::config::AppConfig;

pub struct AppState {
    pub sessions: Arc<dyn ManageSessionUseCase>,
    pub access: Arc<dyn ResolveAccessUseCase>,
    pub config: Arc<AppConfig>,
    pub discord_configured: bool,
}

const SESSION_COOKIE: &str = "ds_session";
const DISCORD_TOKEN_HEADER: &str = "x-discord-token";

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // Libre : le healthcheck du conteneur ne porte pas de jeton et ne
        // divulgue rien.
        .route("/health", get(|| async { "ok" }))
        .route("/auth/discord/authorize", get(authorize))
        .route("/auth/discord/callback", get(callback))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .route("/access", get(access))
        .route("/security/last-logins", get(last_logins))
        .route("/security/purge-logins", post(purge_logins))
        .layer(Extension(state))
}

// ── Helpers HTTP ──────────────────────────────────────────────────────────

fn redirect(location: &str, cookie: Option<&str>) -> Response {
    let mut headers = HeaderMap::new();
    match header::HeaderValue::from_str(location) {
        Ok(v) => headers.insert(header::LOCATION, v),
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "redirection invalide").into_response()
        }
    };
    if let Some(c) = cookie {
        if let Ok(v) = header::HeaderValue::from_str(c) {
            headers.insert(header::SET_COOKIE, v);
        }
    }
    (StatusCode::FOUND, headers).into_response()
}

/// Encode strict selon RFC 3986.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Cookie de session opaque : httpOnly (invisible au JS), Secure, SameSite=Lax
/// (first-party : front et API derrière le même reverse proxy en prod).
fn session_cookie(state: &AppState, id: &str, max_age: i64) -> String {
    let secure = if state.config.cookie_secure {
        " Secure;"
    } else {
        ""
    };
    format!("{SESSION_COOKIE}={id}; HttpOnly;{secure} SameSite=Lax; Path=/; Max-Age={max_age}")
}

fn cleared_cookie(state: &AppState) -> String {
    session_cookie(state, "", 0)
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';').find_map(|kv| {
        let (k, v) = kv.trim().split_once('=')?;
        (k.trim() == name).then(|| v.trim().to_string())
    })
}

fn front_error(state: &AppState, reason: &str) -> Response {
    let front = if state.config.web_front_url.is_empty() {
        "".to_string()
    } else {
        state.config.web_front_url.trim_end_matches('/').to_string()
    };
    redirect(
        &format!("{front}/login?error={}", percent_encode(reason)),
        None,
    )
}

/// Garde des routes de service (`/access`, `/security/*`).
///
/// **Fail-closed** : sans `AUTH_API_TOKEN`, on refuse. L'ancienne version
/// laissait passer, ce qui ouvrait `/access` (resolution de n'importe quel
/// jeton) et `/security/last-logins` (IP, user-agent et identifiants Discord
/// des administrateurs) a tout ce qui joignait le port — le tout signale par un
/// simple `warn!` au demarrage, alors que ce processus est celui qui detient les
/// jetons d'acces. Les autres services du depot refusent de demarrer dans le
/// cas symetrique ; celui-ci refuse de servir.
fn authorize_service(headers: &HeaderMap, state: &AppState) -> Result<(), StatusCode> {
    if state.config.api_token.is_empty() {
        tracing::error!("AUTH_API_TOKEN absent : route de service refusee (voir .env.example)");
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    if service_token_matches(headers, state) {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn service_token_matches(headers: &HeaderMap, state: &AppState) -> bool {
    if state.config.api_token.is_empty() {
        return false;
    }
    let expected = format!("Bearer {}", state.config.api_token);
    let supplied = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    // Constant-time, comme partout ailleurs sur un secret partage.
    bool::from(subtle::ConstantTimeEq::ct_eq(
        supplied.as_bytes(),
        expected.as_bytes(),
    ))
}

/// IP reelle pour la trace de login. Les en-tetes de forwarding ne sont lus
/// que si le jeton interne authentifie la passerelle nginx.
fn client_ip(headers: &HeaderMap, peer_ip: IpAddr, trusted_proxy: bool) -> String {
    if !trusted_proxy {
        return peer_ip.to_string();
    }

    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| peer_ip.to_string())
}

// ── Flux OAuth ────────────────────────────────────────────────────────────

async fn authorize(Extension(state): Extension<Arc<AppState>>) -> Response {
    if !state.discord_configured {
        tracing::error!(
            "OAuth Discord non configure (DISCORD_CLIENT_ID/SECRET/REDIRECT_URI manquants)"
        );
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "OAuth Discord non configure cote serveur",
        )
            .into_response();
    }

    match state.sessions.start_login().await {
        Ok(url) => redirect(&url, None),
        Err(error) => {
            tracing::error!(%error, "demarrage du login impossible");
            (StatusCode::SERVICE_UNAVAILABLE, "login indisponible").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

async fn callback(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> Response {
    // Refus utilisateur, scope invalide… Discord le dit dans la query.
    if let Some(err) = q.error {
        let reason = q.error_description.unwrap_or(err);
        tracing::warn!(%reason, "Discord a renvoye une erreur OAuth");
        return front_error(&state, &reason);
    }

    let (Some(code), Some(csrf)) = (
        q.code.filter(|c| !c.is_empty()),
        q.state.filter(|s| !s.is_empty()),
    ) else {
        return front_error(&state, "parametres_manquants");
    };

    let context = LoginContext {
        client_ip: client_ip(&headers, peer.ip(), service_token_matches(&headers, &state)),
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.chars().take(500).collect())
            .unwrap_or_default(),
    };

    let session = match state.sessions.complete_login(&code, &csrf, context).await {
        Ok(s) => s,
        Err(DomainError::Forbidden(reason)) => {
            tracing::warn!(%reason, "login refuse");
            return front_error(&state, "state_invalide");
        }
        Err(error) => {
            tracing::error!(%error, "login impossible");
            return front_error(&state, "discord_indisponible");
        }
    };

    // Les infos partent dans le FRAGMENT (apres `#`), jamais dans la query :
    // un fragment n'est ni journalise par le serveur, ni transmis en Referer.
    let fragment = format!(
        "token={}&id={}&username={}&global_name={}&avatar={}&is_superadmin={}",
        percent_encode(&session.access_token),
        percent_encode(&session.discord_user_id),
        percent_encode(&session.username),
        percent_encode(session.global_name.as_deref().unwrap_or("")),
        percent_encode(session.avatar.as_deref().unwrap_or("")),
        if session.is_superadmin { "1" } else { "0" },
    );
    let front = state.config.web_front_url.trim_end_matches('/');
    let target = format!("{front}/auth/callback#{fragment}");

    let cookie = session
        .session_id
        .map(|id| session_cookie(&state, &id.to_string(), SESSION_MAX_AGE_SECS));
    redirect(&target, cookie.as_deref())
}

#[derive(Serialize)]
struct SessionResponse {
    token: String,
    id: String,
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
    is_superadmin: bool,
}

fn unauthorized_clearing_cookie(state: &AppState) -> Response {
    let mut headers = HeaderMap::new();
    if let Ok(c) = header::HeaderValue::from_str(&cleared_cookie(state)) {
        headers.insert(header::SET_COOKIE, c);
    }
    (StatusCode::UNAUTHORIZED, headers, "no session").into_response()
}

async fn refresh(Extension(state): Extension<Arc<AppState>>, headers: HeaderMap) -> Response {
    let Some(session_id) =
        cookie_value(&headers, SESSION_COOKIE).and_then(|s| Uuid::parse_str(&s).ok())
    else {
        return unauthorized_clearing_cookie(&state);
    };

    match state.sessions.refresh(session_id).await {
        Ok(s) => Json(SessionResponse {
            token: s.access_token,
            id: s.discord_user_id,
            username: s.username,
            global_name: s.global_name,
            avatar: s.avatar,
            is_superadmin: s.is_superadmin,
        })
        .into_response(),
        // Session inconnue ou revoquee : on efface le cookie, l'utilisateur
        // repasse par le login.
        Err(DomainError::Forbidden(_)) => unauthorized_clearing_cookie(&state),
        // Panne : surtout PAS d'effacement du cookie. Deconnecter tout le monde
        // parce que Discord a hoquete serait le pire des comportements.
        Err(error) => {
            tracing::warn!(%error, "refresh impossible");
            (StatusCode::SERVICE_UNAVAILABLE, "identite indisponible").into_response()
        }
    }
}

async fn logout(Extension(state): Extension<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Some(id) = cookie_value(&headers, SESSION_COOKIE).and_then(|s| Uuid::parse_str(&s).ok())
    {
        if let Err(error) = state.sessions.logout(id).await {
            tracing::warn!(%error, "suppression de session impossible");
        }
    }
    let mut resp_headers = HeaderMap::new();
    if let Ok(c) = header::HeaderValue::from_str(&cleared_cookie(&state)) {
        resp_headers.insert(header::SET_COOKIE, c);
    }
    (StatusCode::NO_CONTENT, resp_headers).into_response()
}

// ── Surface de service ────────────────────────────────────────────────────

/// `GET /access` — cible de l'`auth_request` nginx, et sonde du front.
///
/// Le statut porte toute la réponse : **200** autorisé, **403** identité connue
/// mais hors liste, **401** pas de jeton, **503** impossible de trancher. Le
/// 503 est ce qui distingue une panne d'un refus : nginx ne doit pas laisser
/// croire à l'utilisateur qu'il a perdu ses droits parce que Discord tousse.
///
/// L'identité résolue part en en-tête `X-Auth-User-Id`, pour que l'appelant
/// puisse attribuer une action à son auteur sans refaire la résolution.
async fn access(Extension(state): Extension<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Err(status) = authorize_service(&headers, &state) {
        return status.into_response();
    }

    // Les requetes HTTP du SPA portent encore le token Discord dans un
    // en-tete. Un navigateur ne peut en revanche pas ajouter cet en-tete au
    // handshake WebSocket : dans ce cas, on resout la session opaque portee
    // par le cookie HttpOnly et on garde le token strictement cote serveur.
    let token = headers
        .get(DISCORD_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    let token = match token {
        Some(token) => token,
        None => {
            let Some(session_id) = cookie_value(&headers, SESSION_COOKIE)
                .and_then(|value| Uuid::parse_str(&value).ok())
            else {
                return StatusCode::UNAUTHORIZED.into_response();
            };
            match state.sessions.refresh(session_id).await {
                Ok(session) => session.access_token,
                Err(DomainError::Forbidden(_)) => return StatusCode::UNAUTHORIZED.into_response(),
                Err(error) => {
                    tracing::warn!(%error, "resolution de session impossible");
                    return StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
            }
        }
    };

    match state.access.resolve(&token).await {
        Ok(verdict) if verdict.granted => {
            let mut out = HeaderMap::new();
            if let Ok(v) = header::HeaderValue::from_str(&verdict.discord_user_id) {
                out.insert("x-auth-user-id", v);
            }
            (StatusCode::OK, out).into_response()
        }
        Ok(_) => StatusCode::FORBIDDEN.into_response(),
        Err(DomainError::Forbidden(_)) => StatusCode::FORBIDDEN.into_response(),
        Err(error) => {
            tracing::warn!(%error, "resolution d'identite impossible");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

#[derive(Deserialize)]
struct LimitQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}
fn default_limit() -> i64 {
    50
}

async fn last_logins(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<LimitQuery>,
) -> Response {
    if let Err(status) = authorize_service(&headers, &state) {
        return status.into_response();
    }
    // Borne dure : `limit` vient d'un appelant, et un `LIMIT` non borné sur une
    // table de journal est une dénégation de service gratuite.
    let limit = q.limit.clamp(1, 500);
    match state.sessions.recent_logins(limit).await {
        Ok(rows) => Json(rows).into_response(),
        Err(error) => {
            tracing::warn!(%error, "lecture des logins impossible");
            (StatusCode::SERVICE_UNAVAILABLE, "indisponible").into_response()
        }
    }
}

#[derive(Deserialize)]
struct PurgeQuery {
    days: i32,
}

async fn purge_logins(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<PurgeQuery>,
) -> Response {
    if let Err(status) = authorize_service(&headers, &state) {
        return status.into_response();
    }
    if q.days < 0 {
        return (StatusCode::BAD_REQUEST, "days doit etre positif").into_response();
    }
    match state.sessions.purge_logins(q.days).await {
        Ok(deleted) => Json(serde_json::json!({ "deleted": deleted })).into_response(),
        Err(error) => {
            tracing::warn!(%error, "purge des logins impossible");
            (StatusCode::SERVICE_UNAVAILABLE, "indisponible").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(values: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in values {
            headers.insert(
                header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                header::HeaderValue::from_str(value).unwrap(),
            );
        }
        headers
    }

    #[test]
    fn untrusted_caller_cannot_spoof_forwarded_ip() {
        let peer: IpAddr = "10.0.0.8".parse().unwrap();
        let supplied = headers(&[
            ("x-forwarded-for", "203.0.113.99"),
            ("x-real-ip", "203.0.113.98"),
        ]);

        assert_eq!(client_ip(&supplied, peer, false), "10.0.0.8");
    }

    #[test]
    fn trusted_proxy_forwarded_ip_is_used() {
        let peer: IpAddr = "10.0.0.8".parse().unwrap();
        let supplied = headers(&[("x-forwarded-for", "203.0.113.99")]);

        assert_eq!(client_ip(&supplied, peer, true), "203.0.113.99");
    }

    #[test]
    fn trusted_proxy_with_invalid_header_falls_back_to_peer() {
        let peer: IpAddr = "10.0.0.8".parse().unwrap();
        let supplied = headers(&[("x-forwarded-for", "")]);

        assert_eq!(client_ip(&supplied, peer, true), "10.0.0.8");
    }
}
