//! Streams Redis dediees aux logs systeme (Logs systeme = page web a 4
//! colonnes : bot, worker, api, websocket — plus la categorie historique
//! `discord`).
//!
//! Pourquoi : avant ce module, *tous* les logs (info inclus) etaient
//! persistes en Postgres via `logs`. La categorie `discord`, tres bavarde,
//! noyait `bot` / `worker` / `websocket` cote frontend (cap a 200 lignes
//! cote API). Resultat : la colonne "Bots" arrivait quasi-vide.
//!
//! Solution : chaque categorie a sa propre stream `logs:{category}` avec
//! `MAXLEN ~ STREAM_MAXLEN` (cap auto, ~ = approx pour O(1) amorti).
//! L'API XADD a chaque ecriture, et XREVRANGE pour la lecture page.
//! Postgres ne recoit plus que les `warn` / `error` (forensics long terme,
//! recherche par guild). Les `info` vivent dans Redis (jusqu'a STREAM_MAXLEN
//! par categorie).
//!
//! Cle Redis : `logs:{category}` (ex `logs:bot`, `logs:worker`, ...).
//! Field unique du stream : `data` (JSON serialise du LogEntry).

use redis::Client;
use tracing::warn;

use ops_core::domain::entities::log_entry::LogEntry;

/// Borne de taille approximative par stream (par categorie).
/// 5000 lignes × ~5 categories = ~25k entrees max en RAM Redis.
pub const STREAM_MAXLEN: usize = 5_000;

/// Limite par defaut pour les XREVRANGE de page.
pub const DEFAULT_READ_LIMIT: usize = 200;

const FIELD: &str = "data";

fn stream_key(category: &str) -> String {
    format!("logs:{category}")
}

/// Append un log sur la stream de sa categorie. Fire-and-forget : si
/// Redis est down, on log un warn et on continue. Les warns/errors
/// passent egalement par Postgres (cf handler), donc rien n'est perdu
/// pour la forensique.
pub async fn xadd_log(client: &Client, entry: &LogEntry) {
    let json = match serde_json::to_string(entry) {
        Ok(j) => j,
        Err(e) => {
            warn!(error = %e, "Echec serialisation LogEntry pour Redis stream");
            return;
        }
    };
    let key = stream_key(&entry.category);

    let mut conn = match client.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "Echec connexion Redis pour log stream XADD");
            return;
        }
    };

    let res: redis::RedisResult<String> = redis::cmd("XADD")
        .arg(&key)
        .arg("MAXLEN")
        .arg("~")
        .arg(STREAM_MAXLEN)
        .arg("*")
        .arg(FIELD)
        .arg(&json)
        .query_async(&mut conn)
        .await;
    if let Err(e) = res {
        warn!(error = %e, key = %key, "Echec Redis XADD log");
    }
}

/// Lit les `limit` derniers logs de la categorie (les plus recents
/// d'abord), avec un filtre de niveau optionnel ("info" | "warn" |
/// "error"). On surconsomme le stream pour pouvoir filtrer post-XREVRANGE
/// sans perdre la page demandee. En pratique : on lit `limit * 4` (cap a
/// MAXLEN), on filtre, puis on tronque a `limit`.
pub async fn xrevrange_logs(
    client: &Client,
    category: &str,
    level: Option<&str>,
    limit: usize,
) -> Vec<LogEntry> {
    let key = stream_key(category);
    let read_count = (limit.saturating_mul(if level.is_some() { 4 } else { 1 })).min(STREAM_MAXLEN);

    let mut conn = match client.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "Echec connexion Redis pour log stream XREVRANGE");
            return Vec::new();
        }
    };

    // XREVRANGE key + - COUNT n  -> Vec<(id, Vec<(field, value)>)>
    let raw: redis::RedisResult<Vec<(String, Vec<(String, String)>)>> = redis::cmd("XREVRANGE")
        .arg(&key)
        .arg("+")
        .arg("-")
        .arg("COUNT")
        .arg(read_count)
        .query_async(&mut conn)
        .await;

    let raw = match raw {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, key = %key, "Echec Redis XREVRANGE log");
            return Vec::new();
        }
    };

    let mut out = Vec::with_capacity(raw.len().min(limit));
    for (_id, fields) in raw {
        let json = fields
            .iter()
            .find_map(|(k, v)| (k == FIELD).then_some(v.as_str()));
        let Some(json) = json else { continue };
        let Ok(entry) = serde_json::from_str::<LogEntry>(json) else {
            continue;
        };
        if let Some(lv) = level {
            if entry.level != lv {
                continue;
            }
        }
        out.push(entry);
        if out.len() >= limit {
            break;
        }
    }
    out
}

/// Vide la stream d'une categorie (utilise par DELETE /api/logs/{cat}).
pub async fn delete_stream(client: &Client, category: &str) {
    let key = stream_key(category);
    let mut conn = match client.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "Echec connexion Redis pour DEL stream");
            return;
        }
    };
    let res: redis::RedisResult<i64> = redis::cmd("DEL").arg(&key).query_async(&mut conn).await;
    if let Err(e) = res {
        warn!(error = %e, key = %key, "Echec Redis DEL stream");
    }
}
