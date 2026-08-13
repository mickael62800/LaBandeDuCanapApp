//! Helpers generiques pour le pattern cache-aside JSON.
//!
//! Evite de repeter le boilerplate `serde_json::{from_str, to_string}` +
//! `get_json` / `set_json` dans chaque service applicatif.
//!
//! Usage typique :
//!
//! ```ignore
//! let tickets = cached_json(&self.cache, &cache_key, TTL, || async {
//!     self.ticket_repo.find_all(...).await
//! }).await?;
//! ```

use std::future::Future;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::outbound::system::cache::CachePort;

/// Pattern cache-aside : lit depuis le cache, sinon execute `fetch` et ecrit
/// le resultat dans le cache avec le TTL specifie.
///
/// Semantique :
/// - Un echec Redis `GET` propage l'erreur (comportement existant des services).
/// - Un JSON invalide en cache est silencieusement ignore, on fallback sur `fetch`.
/// - Un echec Redis `SETEX` est logue mais n'empeche pas le retour de la valeur.
/// - Un echec de serialisation JSON est silencieusement ignore (pas de set).
pub async fn cached_json<T, F, Fut>(
    cache: &Arc<dyn CachePort>,
    key: &str,
    ttl_secs: u64,
    fetch: F,
) -> Result<T, DomainError>
where
    T: Serialize + DeserializeOwned,
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, DomainError>>,
{
    if let Some(json) = cache.get_json(key).await? {
        if let Ok(data) = serde_json::from_str::<T>(&json) {
            return Ok(data);
        }
    }

    let data = fetch().await?;

    if let Ok(json) = serde_json::to_string(&data) {
        if let Err(e) = cache.set_json(key, &json, ttl_secs).await {
            tracing::warn!(error = %e, cache_key = %key, "Echec cache set (cached_json)");
        }
    }

    Ok(data)
}

#[cfg(test)]
#[path = "tests/cache_helpers.rs"]
mod tests;
