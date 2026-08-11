use redis::AsyncCommands;
use serde::Serialize;
use sqlx::PgPool;
use tracing::{info, warn};

use platform_common_worker::is_worker_enabled;

const CACHE_TTL_SECS: u64 = 7200; // 2 hours
const DAYS: i32 = 30;
const LIMIT: i64 = 20;

#[derive(sqlx::FromRow)]
struct GuildRow {
    guild_id: String,
}

#[derive(sqlx::FromRow, Serialize)]
struct VoiceChannelStats {
    channel_id: Option<String>,
    channel_name: Option<String>,
    total_sessions: Option<i64>,
    total_seconds: Option<i64>,
    avg_seconds: Option<f64>,
}

/// Pre-compute voice channel statistics per guild and store in Redis.
pub async fn run(pool: &PgPool, redis: &redis::Client) -> Result<(), String> {
    let guilds: Vec<GuildRow> =
        sqlx::query_as::<_, GuildRow>("SELECT guild_id FROM guilds ORDER BY name")
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Query guilds: {e}"))?;

    if guilds.is_empty() {
        return Ok(());
    }

    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("Redis connection: {e}"))?;

    let mut count = 0u64;

    for guild in &guilds {
        if !is_worker_enabled(pool, &guild.guild_id, "cache").await {
            continue;
        }

        match compute_voice_stats(pool, &guild.guild_id).await {
            Ok(stats) => {
                let json = serde_json::to_string(&stats)
                    .map_err(|e| format!("Serialize voice stats: {e}"))?;

                let key = format!("voice_stats:{}:{}:{}", guild.guild_id, DAYS, LIMIT);
                let _: () = conn
                    .set_ex(&key, &json, CACHE_TTL_SECS)
                    .await
                    .map_err(|e| format!("Redis SET {key}: {e}"))?;

                count += 1;
            }
            Err(e) => {
                warn!(error = %e, guild = %guild.guild_id, "Erreur calcul voice stats cache");
            }
        }
    }

    if count > 0 {
        info!(guilds = count, "Cache voice stats rechauffe");
    }

    Ok(())
}

async fn compute_voice_stats(
    pool: &PgPool,
    guild_id: &str,
) -> Result<Vec<VoiceChannelStats>, String> {
    sqlx::query_as::<_, VoiceChannelStats>(
        "SELECT channel_id, channel_name, \
                COUNT(*) AS total_sessions, \
                SUM(duration_secs)::bigint AS total_seconds, \
                AVG(duration_secs)::float8 AS avg_seconds \
         FROM voice_sessions \
         WHERE guild_id = $1 AND created_at >= NOW() - INTERVAL '30 days' \
         GROUP BY channel_id, channel_name \
         ORDER BY total_seconds DESC \
         LIMIT $2",
    )
    .bind(guild_id)
    .bind(LIMIT)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("voice_stats: {e}"))
}
