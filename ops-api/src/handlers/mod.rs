//! Adaptateurs entrants HTTP.

pub mod alert_rules;
pub mod docker;
pub mod metrics;
pub mod security;
pub mod server_events;

use axum::Json;

/// Acquittement uniforme des actions sans corps utile (start/stop/prune...).
/// Le front n'en lit que le succes HTTP, mais un corps vide casse certains
/// clients.
pub fn ok_response() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}
/// Borne une limite de pagination fournie par le client.
///
/// Sans borne haute, un `?limit=100000` ferait remonter toute une table dans
/// une reponse HTTP ; sans borne basse, un `0` rendrait une page vide sans
/// que l'appelant comprenne pourquoi.
pub fn normalize_in(value: Option<i64>, defaut: i64, min: i64, max: i64) -> i64 {
    value.unwrap_or(defaut).clamp(min, max)
}