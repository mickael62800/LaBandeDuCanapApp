//! Phase 6A — Refresh periodique du cache watched_users.
//!
//! Avant la Phase 6A, audit-bot faisait une boucle `sleep(60s) + API call`
//! dans `handler/mod.rs`. Ce pattern ne scale pas horizontalement (N replicas
//! d'audit-bot = N appels API inutiles).
//!
//! Ce worker extrait le refresh :
//!   1. Query Postgres direct (plus efficace que l'API) : UNION des user_ids
//!      avec infractions + ceux dans manual_watched_users
//!   2. Push le snapshot dans Redis sous la cle `audit:watched_users` (JSON array)
//!      avec TTL 5 min (fail-safe si le worker est down)
//!   3. Publie un event `watched_users_refreshed` sur la stream `sentinel:events`
//!      (Phase 5B) pour que les replicas d'audit-bot consument et mettent a
//!      jour leur cache in-memory DashSet
//!
//! Pattern identique a temp-roles-worker / appeal-sla-worker : UPDATE +
//! XADD sur `sentinel:events` avec un event type dedie.

use sqlx::PgPool;
use tracing::{debug, info, warn};

const REDIS_KEY: &str = "audit:watched_users";
const REDIS_TTL_SECS: u64 = 300;

pub async fn run(
    pool: &PgPool,
    redis: &redis::aio::ConnectionManager,
    query_limit: i64,
) -> Result<(), String> {
    // 1. Query Postgres : union des user_ids avec infractions + manual.
    // `query_limit` est borne (100..100000) cote WorkerConfig, on peut donc
    // l'injecter directement dans le LIMIT.
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT user_id FROM ( \
             SELECT DISTINCT user_id FROM infractions \
             UNION \
             SELECT DISTINCT user_id FROM manual_watched_users \
         ) AS u \
         LIMIT $1",
    )
    .bind(query_limit)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query watched users: {e}"))?;

    let user_ids: Vec<String> = rows.into_iter().map(|(id,)| id).collect();
    let count = user_ids.len();

    debug!(count, "watched users snapshot");

    let serialized =
        serde_json::to_string(&user_ids).map_err(|e| format!("serialize user_ids: {e}"))?;

    // 2. Push dans Redis (SET avec TTL)
    let mut conn = redis.clone();

    use redis::AsyncCommands;
    conn.set_ex::<_, _, ()>(REDIS_KEY, &serialized, REDIS_TTL_SECS)
        .await
        .map_err(|e| format!("redis set_ex: {e}"))?;

    // 3. Publie l'event sur la stream sentinel:events (pattern Phase 5B)
    let event_payload = serde_json::json!({
        "event": "watched_users_refreshed",
        "data": {
            "count": count,
            "ttl_secs": REDIS_TTL_SECS,
        }
    });

    let event_str = event_payload.to_string();
    let res = platform_common_worker::redis_helpers::xadd_event(&mut conn, &event_str).await;

    if let Err(e) = res {
        warn!(error = %e, "XADD watched_users_refreshed failed");
    }

    info!(count, "watched_users cache refreshed");
    Ok(())
}
