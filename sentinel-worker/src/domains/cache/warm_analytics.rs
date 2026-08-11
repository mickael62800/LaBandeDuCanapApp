use redis::AsyncCommands;
use serde::Serialize;
use sqlx::PgPool;
use tracing::{info, warn};

use platform_common_worker::is_worker_enabled;

const CACHE_TTL_SECS: u64 = 600; // 10 minutes
const DAYS: i32 = 30;
const LIMIT: i64 = 10;

#[derive(sqlx::FromRow)]
struct GuildRow {
    guild_id: String,
}

#[derive(Serialize)]
struct AnalyticsBundle {
    heatmap: Vec<HeatmapRow>,
    action_distribution: Vec<ActionDistributionRow>,
    top_infractors: Vec<TopInfractorRow>,
    moderation_trend: Vec<ModerationTrendRow>,
    peak_hours: Vec<PeakHourRow>,
}

#[derive(sqlx::FromRow, Serialize)]
struct HeatmapRow {
    day_of_week: Option<i32>,
    hour: Option<i32>,
    count: Option<i64>,
}

#[derive(sqlx::FromRow, Serialize)]
struct ActionDistributionRow {
    action: Option<String>,
    count: Option<i64>,
}

#[derive(sqlx::FromRow, Serialize)]
struct TopInfractorRow {
    user_id: Option<String>,
    count: Option<i64>,
}

#[derive(sqlx::FromRow, Serialize)]
struct ModerationTrendRow {
    day: Option<chrono::NaiveDate>,
    count: Option<i64>,
}

#[derive(sqlx::FromRow, Serialize)]
struct PeakHourRow {
    hour: Option<i32>,
    count: Option<i64>,
}

/// Pre-compute full analytics bundle for each guild and store in Redis.
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

        match compute_analytics(pool, &guild.guild_id).await {
            Ok(bundle) => {
                let json = serde_json::to_string(&bundle)
                    .map_err(|e| format!("Serialize analytics: {e}"))?;

                let key = format!("analytics:full:{}:{}:{}", guild.guild_id, DAYS, LIMIT);
                let _: () = conn
                    .set_ex(&key, &json, CACHE_TTL_SECS)
                    .await
                    .map_err(|e| format!("Redis SET {key}: {e}"))?;

                count += 1;
            }
            Err(e) => {
                warn!(error = %e, guild = %guild.guild_id, "Erreur calcul analytics cache");
            }
        }
    }

    if count > 0 {
        info!(guilds = count, "Cache analytics rechauffe");
    }

    Ok(())
}

async fn compute_analytics(pool: &PgPool, guild_id: &str) -> Result<AnalyticsBundle, String> {
    let heatmap = sqlx::query_as::<_, HeatmapRow>(
        "SELECT EXTRACT(DOW FROM created_at)::int AS day_of_week, \
                EXTRACT(HOUR FROM created_at)::int AS hour, \
                COUNT(*) AS count \
         FROM infractions \
         WHERE guild_id = $1 AND created_at >= NOW() - INTERVAL '30 days' \
         GROUP BY day_of_week, hour \
         ORDER BY day_of_week, hour",
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("heatmap: {e}"))?;

    let action_distribution = sqlx::query_as::<_, ActionDistributionRow>(
        "SELECT action, COUNT(*) AS count \
         FROM infractions \
         WHERE guild_id = $1 AND created_at >= NOW() - INTERVAL '30 days' \
         GROUP BY action \
         ORDER BY count DESC",
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("action_distribution: {e}"))?;

    let top_infractors = sqlx::query_as::<_, TopInfractorRow>(
        "SELECT user_id, COUNT(*) AS count \
         FROM infractions \
         WHERE guild_id = $1 AND created_at >= NOW() - INTERVAL '30 days' \
         GROUP BY user_id \
         ORDER BY count DESC \
         LIMIT $2",
    )
    .bind(guild_id)
    .bind(LIMIT)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("top_infractors: {e}"))?;

    let moderation_trend = sqlx::query_as::<_, ModerationTrendRow>(
        "SELECT created_at::date AS day, COUNT(*) AS count \
         FROM infractions \
         WHERE guild_id = $1 AND created_at >= NOW() - INTERVAL '30 days' \
         GROUP BY day \
         ORDER BY day",
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("moderation_trend: {e}"))?;

    let peak_hours = sqlx::query_as::<_, PeakHourRow>(
        "SELECT EXTRACT(HOUR FROM created_at)::int AS hour, COUNT(*) AS count \
         FROM infractions \
         WHERE guild_id = $1 AND created_at >= NOW() - INTERVAL '30 days' \
         GROUP BY hour \
         ORDER BY count DESC",
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("peak_hours: {e}"))?;

    Ok(AnalyticsBundle {
        heatmap,
        action_distribution,
        top_infractors,
        moderation_trend,
        peak_hours,
    })
}
