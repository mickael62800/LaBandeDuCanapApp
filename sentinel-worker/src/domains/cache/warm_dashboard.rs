use redis::AsyncCommands;
use serde::Serialize;
use sqlx::PgPool;
use tracing::{info, warn};

use platform_common_worker::is_worker_enabled;

const CACHE_TTL_SECS: u64 = 900; // 15 minutes

#[derive(sqlx::FromRow)]
struct GuildRow {
    guild_id: String,
}

#[derive(Serialize)]
struct DashboardOverview {
    total_infractions: i64,
    unique_users: i64,
    recent_infractions: i64,
    active_members: i64,
}

/// Pre-compute dashboard overview stats per guild and store in Redis.
pub async fn run(pool: &PgPool, redis: &redis::aio::ConnectionManager) -> Result<(), String> {
    let guilds: Vec<GuildRow> =
        sqlx::query_as::<_, GuildRow>("SELECT guild_id FROM guilds ORDER BY name")
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Query guilds: {e}"))?;

    if guilds.is_empty() {
        return Ok(());
    }

    let mut conn = redis.clone();

    let mut count = 0u64;

    for guild in &guilds {
        if !is_worker_enabled(pool, &guild.guild_id, "cache").await {
            continue;
        }

        match compute_dashboard(pool, &guild.guild_id).await {
            Ok(overview) => {
                let json = serde_json::to_string(&overview)
                    .map_err(|e| format!("Serialize dashboard: {e}"))?;

                let key = format!("stats:overview:{}", guild.guild_id);
                let _: () = conn
                    .set_ex(&key, &json, CACHE_TTL_SECS)
                    .await
                    .map_err(|e| format!("Redis SET {key}: {e}"))?;

                count += 1;
            }
            Err(e) => {
                warn!(error = %e, guild = %guild.guild_id, "Erreur calcul dashboard cache");
            }
        }
    }

    if count > 0 {
        info!(guilds = count, "Cache dashboard rechauffe");
    }

    Ok(())
}

async fn compute_dashboard(pool: &PgPool, guild_id: &str) -> Result<DashboardOverview, String> {
    let total_infractions: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM infractions WHERE guild_id = $1")
            .bind(guild_id)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("total_infractions: {e}"))?;

    let unique_users: (i64,) =
        sqlx::query_as("SELECT COUNT(DISTINCT user_id) FROM user_stats WHERE guild_id = $1")
            .bind(guild_id)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("unique_users: {e}"))?;

    let recent_infractions: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= NOW() - INTERVAL '7 days'",
    )
    .bind(guild_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("recent_infractions: {e}"))?;

    let active_members: (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT user_id) FROM user_stats WHERE guild_id = $1 AND updated_at >= NOW() - INTERVAL '7 days'",
    )
    .bind(guild_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("active_members: {e}"))?;

    Ok(DashboardOverview {
        total_infractions: total_infractions.0,
        unique_users: unique_users.0,
        recent_infractions: recent_infractions.0,
        active_members: active_members.0,
    })
}
