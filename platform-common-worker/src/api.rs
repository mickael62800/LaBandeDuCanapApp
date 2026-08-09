//! Helpers d'appel HTTP vers l'API Sentinel.
//!
//! Centralise le boilerplate `reqwest::Client::new() + bearer_auth(API_KEY)
//! + send + parse JSON` qui etait duplique dans plusieurs jobs (conduct_regen,
//!   sync_ban_proposals, etc.).
//!
//! `API_URL` (default `http://localhost:3000`) et `API_KEY` (optional) sont
//! lus depuis l'environnement.

use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;

const DEFAULT_API_URL: &str = "http://localhost:3000";

/// Construit l'URL complete a partir d'un path relatif (`/api/...`).
fn full_url(path: &str) -> String {
    let base = std::env::var("API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());
    format!("{base}{path}")
}

fn api_key() -> String {
    std::env::var("SENTINEL_API_KEY").unwrap_or_default()
}

fn add_auth(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let key = api_key();
    if key.is_empty() {
        req
    } else {
        req.bearer_auth(key)
    }
}

/// `POST {API_URL}{path}` avec body JSON, Bearer auth optionnel, retour
/// JSON parse en `T`. Encapsule le pattern HTTP recurrent des workers.
///
/// Errors :
/// - `HTTP send: ...` : echec reseau
/// - `HTTP {status}: {body}` : reponse non-2xx
/// - `HTTP parse: ...` : reponse 2xx avec body non-deserialisable
pub async fn post_json<B, T>(path: &str, body: &B) -> Result<T, String>
where
    B: Serialize + ?Sized,
    T: DeserializeOwned,
{
    let url = full_url(path);
    let client = Client::new();
    let req = client.post(&url).json(body);
    let resp = add_auth(req)
        .send()
        .await
        .map_err(|e| format!("HTTP send: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {status}: {body}"));
    }
    resp.json::<T>()
        .await
        .map_err(|e| format!("HTTP parse: {e}"))
}

/// Variante de `post_json` sans body (POST avec `{}`). Pratique pour les
/// endpoints "tick" stateless.
pub async fn post_empty<T>(path: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    post_json(path, &serde_json::json!({})).await
}
