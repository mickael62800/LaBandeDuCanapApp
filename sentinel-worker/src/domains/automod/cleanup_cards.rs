//! Suppression des cartes de review automod closes depuis plus d'un mois.
//!
//! Job 24h : appelle `POST /api/automod/cleanup-expired-cards`. L'API trouve
//! les reviews closes (applied|ignored) resolues il y a plus de 30 jours et
//! encore mappees a un message Discord, broadcast un event `automod_card_expired`
//! (le bot supprime le message) et retire le mapping. La review + le transcript
//! restent en DB (la trace consultable sur le web est conservee).

use sqlx::PgPool;
use tracing::{info, warn};

use platform_common_worker::api;

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    let body = serde_json::json!({ "days": 30 });
    match api::post_json::<_, serde_json::Value>("/api/automod/cleanup-expired-cards", &body).await
    {
        Ok(v) => {
            let n = v.get("expired").and_then(|x| x.as_u64()).unwrap_or(0);
            if n > 0 {
                info!(expired = n, "Cartes automod closes (>1 mois) supprimees");
            }
            Ok(())
        }
        Err(e) => {
            warn!(error = %e, "echec cleanup cartes automod");
            Err(e)
        }
    }
}

