//! Helpers Redis pour les workers : init du client + XADD vers les streams.

const STREAM_KEY: &str = "sentinel:events";
const STREAM_MAXLEN: usize = 10_000;
const PAYLOAD_FIELD: &str = "payload";

/// Ouvre un `redis::Client` ou `exit(1)` si l'URL est invalide. Pattern
/// utilise par TOUS les `main.rs` des workers (8 sites duplicat).
pub fn open_or_exit(redis_url: &str) -> redis::Client {
    match redis::Client::open(redis_url) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Impossible de creer le client Redis");
            std::process::exit(1);
        }
    }
}

/// Publie un event sur la stream `sentinel:events` (XADD avec MAXLEN ~).
/// Best-effort : retourne `Err` si la commande echoue, le caller log/skip.
///
/// Format attendu : `payload` est le JSON serialise du struct
/// `{ "event": "...", "data": {...} }` que les consumers parsent.
pub async fn xadd_event<C>(conn: &mut C, payload: &str) -> redis::RedisResult<String>
where
    C: redis::aio::ConnectionLike + Send + Unpin,
{
    redis::cmd("XADD")
        .arg(STREAM_KEY)
        .arg("MAXLEN")
        .arg("~")
        .arg(STREAM_MAXLEN)
        .arg("*")
        .arg(PAYLOAD_FIELD)
        .arg(payload)
        .query_async(conn)
        .await
}

/// Variante qui prend un `serde_json::Value` deja construit et serialise
/// avant XADD. Retourne `Err` si la serialisation OU le XADD echoue.
pub async fn xadd_event_json<C>(conn: &mut C, payload: &serde_json::Value) -> Result<(), String>
where
    C: redis::aio::ConnectionLike + Send + Unpin,
{
    let serialized = serde_json::to_string(payload).map_err(|e| format!("serialize: {e}"))?;
    xadd_event(conn, &serialized)
        .await
        .map_err(|e| format!("XADD: {e}"))?;
    Ok(())
}
