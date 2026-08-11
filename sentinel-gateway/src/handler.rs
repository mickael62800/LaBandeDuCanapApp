use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use subtle::ConstantTimeEq;
use tracing::{info, warn};

use crate::broadcaster::EventBroadcaster;
use crate::logger::GatewayLogger;

/// WebSocket close code: "Try Again Later" (server at capacity)
const WS_CLOSE_TRY_AGAIN_LATER: u16 = 1013;

#[derive(Clone)]
pub struct GatewayState {
    pub broadcaster: Arc<EventBroadcaster>,
    pub api_key: String,
    pub auth_api_url: String,
    pub auth_api_token: String,
    pub logger: Arc<GatewayLogger>,
    pub http_client: reqwest::Client,
}

/// Valide le cookie de session HttpOnly aupres d'auth-api. Le navigateur
/// l'ajoute automatiquement au handshake same-origin, sans exposer de jeton
/// dans l'URL, les logs nginx ou l'historique.
async fn session_cookie_authorized(state: &GatewayState, headers: &HeaderMap) -> bool {
    let Some(cookie) = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    if state.auth_api_token.is_empty() {
        warn!("AUTH_API_TOKEN absent -> authentification WebSocket par session impossible");
        return false;
    }

    let url = format!("{}/access", state.auth_api_url.trim_end_matches('/'));
    match state
        .http_client
        .get(url)
        .bearer_auth(&state.auth_api_token)
        .header(header::COOKIE, cookie)
        .send()
        .await
    {
        Ok(response) => response.status().is_success(),
        Err(error) => {
            warn!(%error, "auth-api inaccessible -> refus du WebSocket");
            false
        }
    }
}

fn service_bearer_authorized(state: &GatewayState, headers: &HeaderMap) -> bool {
    if state.api_key.is_empty() {
        return false;
    }
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|token| token.as_bytes().ct_eq(state.api_key.as_bytes()).into())
        .unwrap_or(false)
}

/// Handler WebSocket — cookie HttpOnly pour le Web, header Bearer pour les
/// eventuels clients internes. Aucun credential n'est lu depuis l'URL.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    State(state): State<GatewayState>,
) -> Response {
    // Aucun credential n'est accepte dans la query string. Les utilisateurs
    // Web passent par le cookie HttpOnly ; un eventuel client interne peut
    // utiliser le header Authorization standard.
    let valid_service = service_bearer_authorized(&state, &headers);
    let valid_session = if valid_service {
        false
    } else {
        session_cookie_authorized(&state, &headers).await
    };
    if !valid_service && !valid_session {
        warn!(client_ip = %addr, "WebSocket rejected: no valid auth (session or service bearer)");
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let logger = state.logger.clone();
    let client_ip = addr.to_string();
    ws.on_upgrade(move |socket| handle_socket(socket, state.broadcaster, logger, client_ip))
}

// Le `if let` imbrique ne peut pas fusionner avec le match : `data` y est deplace.
#[allow(clippy::collapsible_match)]
async fn handle_socket(
    mut socket: WebSocket,
    broadcaster: Arc<EventBroadcaster>,
    logger: Arc<GatewayLogger>,
    client_ip: String,
) {
    // Verifier la limite de connexions
    let mut rx = match broadcaster.subscribe() {
        Some(rx) => rx,
        None => {
            warn!(client_ip = %client_ip, connected = broadcaster.connected_count(), "WebSocket rejected: max connections reached");
            if let Err(e) = socket
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: WS_CLOSE_TRY_AGAIN_LATER,
                    reason: "Too many connections".into(),
                })))
                .await
            {
                warn!(error = %e, "Failed to send close frame");
            }
            return;
        }
    };

    let clients = broadcaster.connected_count();
    info!(clients, client_ip = %client_ip, "WebSocket client connected");
    logger.info(
        "Client WebSocket connecte",
        serde_json::json!({
            "event_type": "websocket.client_connected",
            "client_ip": &client_ip,
            "total_clients": clients,
        }),
    );

    let mut events_relayed: u64 = 0;
    let mut events_skipped: u64 = 0;

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(ws_event) => {
                        match serde_json::to_string(&ws_event) {
                            Ok(json) => {
                                if socket.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                                events_relayed += 1;
                            }
                            Err(e) => {
                                warn!(error = %e, event_type = %ws_event.event, "Failed to serialize event");
                                logger.warn("Echec serialisation event", serde_json::json!({
                                    "event_type": "websocket.serialize_error",
                                    "error": e.to_string(),
                                    "ws_event_type": ws_event.event,
                                    "client_ip": &client_ip,
                                }));
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, client_ip = %client_ip, "Client lagged");
                        events_skipped += n;
                        logger.warn("Client lagged (events skip)", serde_json::json!({
                            "event_type": "websocket.client_lagged",
                            "skipped": n,
                            "client_ip": &client_ip,
                        }));
                    }
                    Err(_) => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    broadcaster.unsubscribe();
    let clients = broadcaster.connected_count();
    info!(clients, client_ip = %client_ip, events_relayed, events_skipped, "WebSocket client disconnected");
    logger.info(
        "Client WebSocket deconnecte",
        serde_json::json!({
            "event_type": "websocket.client_disconnected",
            "client_ip": &client_ip,
            "total_clients": clients,
            "events_relayed": events_relayed,
            "skipped_events": events_skipped,
        }),
    );
}
