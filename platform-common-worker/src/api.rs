//! Helpers d'appel HTTP vers l'API Sentinel.
//!
//! Facade Sentinel autour du transport partage [`HttpJobClient`]. Elle garde
//! les fonctions historiques utilisees par les jobs, sans dupliquer la gestion
//! des URL, du Bearer, des statuts HTTP et du decodage JSON.
//!
//! `API_URL` (defaut `http://localhost:3000`) et `SENTINEL_API_KEY` (optionnel)
//! sont lus une fois depuis l'environnement au premier appel.

use std::sync::OnceLock;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::http_job::HttpJobClient;

const DEFAULT_API_URL: &str = "http://localhost:3000";

static API_CLIENT: OnceLock<HttpJobClient> = OnceLock::new();

fn client() -> &'static HttpJobClient {
    API_CLIENT.get_or_init(|| {
        let base_url = std::env::var("API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());
        let token = std::env::var("SENTINEL_API_KEY").unwrap_or_default();
        HttpJobClient::new(base_url, token, Duration::from_secs(30))
    })
}

/// `POST {API_URL}{path}` avec body JSON, Bearer auth optionnel, retour
/// JSON parse en `T`. Encapsule le pattern HTTP recurrent des workers.
///
/// Errors :
/// - `HTTP send: ...` : echec reseau
/// - `HTTP {status}: {body}` : reponse non-2xx
/// - `decode reponse: ...` : reponse 2xx avec body non-deserialisable
pub async fn post_json<B, T>(path: &str, body: &B) -> Result<T, String>
where
    B: Serialize + ?Sized,
    T: DeserializeOwned,
{
    client().post_json_body(path, body).await
}

/// Variante de `post_json` sans body (POST avec `{}`). Pratique pour les
/// endpoints "tick" stateless.
pub async fn post_empty<T>(path: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    post_json(path, &serde_json::json!({})).await
}
