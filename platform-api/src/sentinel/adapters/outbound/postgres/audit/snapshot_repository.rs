//! Adapter sortant Postgres des jobs analytics (snapshots quotidien/horaire,
//! purge de retention, liste des guilds). Tout le SQL brut des jobs vit ici ;
//! le use case reste pur. Les `format!` avec `anchor_hour` sont surs :
//! `anchor_hour` est clampe 0..23 par l'appelant (pas d'injection).

use async_trait::async_trait;
use sqlx::PgPool;

use crate::sentinel::adapters::outbound::postgres::pg_err;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::audit::snapshot_repository::SnapshotRepository;

pub struct PgSnapshotRepository {
    pool: PgPool,
}

impl PgSnapshotRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SnapshotRepository for PgSnapshotRepository {
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT guild_id FROM guilds ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(|(g,)| g).collect())
    }

    async fn snapshot_daily(
        &self,
        guild_id: &str,
        track_messages: bool,
        track_voice: bool,
        anchor_hour: i64,
    ) -> Result<(), DomainError> {
        // Le "stat day" courant : si l'heure UTC >= anchor, on est dans le jour
        // courant ; sinon on est encore dans le jour precedent (la baseline n'a
        // pas encore tourne). anchor_hour est clampe 0..23 par le use case.
        let stat_day_expr = format!(
            "CASE WHEN EXTRACT(HOUR FROM NOW())::int >= {anchor_hour} \
             THEN CURRENT_DATE ELSE CURRENT_DATE - 1 END"
        );

        // Step 1 : ON CONFLICT DO NOTHING -> on n'ecrase JAMAIS une baseline deja
        // capturee pour ce jour.
        let baseline_sql = format!(
            "INSERT INTO analytics_daily_baseline (guild_id, day, total_messages, total_voice_seconds) \
             SELECT $1, ({stat_day_expr}), \
                    COALESCE((SELECT SUM(message_count) FROM user_stats WHERE guild_id = $1), 0), \
                    COALESCE((SELECT SUM(voice_seconds) FROM user_stats WHERE guild_id = $1), 0) \
             ON CONFLICT (guild_id, day) DO NOTHING"
        );
        sqlx::query(&baseline_sql)
            .bind(guild_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        // Step 2 : delta = total_now - baseline[stat_day].total_*
        let msg_expr = if track_messages {
            format!(
                "GREATEST(COALESCE((SELECT SUM(message_count) FROM user_stats WHERE guild_id = $1), 0) \
                 - COALESCE((SELECT total_messages FROM analytics_daily_baseline WHERE guild_id = $1 AND day = ({stat_day_expr})), 0), 0)"
            )
        } else {
            "0".to_string()
        };
        let voice_expr = if track_voice {
            format!(
                "GREATEST(COALESCE((SELECT SUM(voice_seconds) / 60 FROM user_stats WHERE guild_id = $1), 0) \
                 - COALESCE((SELECT total_voice_seconds / 60 FROM analytics_daily_baseline WHERE guild_id = $1 AND day = ({stat_day_expr})), 0), 0)"
            )
        } else {
            "0".to_string()
        };

        let sql = format!(
            "INSERT INTO daily_activity (guild_id, day, messages, voice_minutes, active_members, new_members, leaves, infractions, warns, mutes, bans) \
             SELECT $1, ({stat_day_expr}), \
               {msg_expr}, \
               {voice_expr}, \
               COALESCE((SELECT COUNT(DISTINCT user_id) FROM user_stats WHERE guild_id = $1 AND updated_at >= ({stat_day_expr})), 0)::integer, \
               COALESCE((SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1 AND event_type = 'member_join' AND created_at >= ({stat_day_expr}))::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1 AND event_type = 'member_leave' AND created_at >= ({stat_day_expr}))::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= ({stat_day_expr}))::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= ({stat_day_expr}) AND action = 'warn')::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= ({stat_day_expr}) AND action = 'mute')::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= ({stat_day_expr}) AND action = 'ban')::integer, 0) \
             ON CONFLICT (guild_id, day) DO UPDATE SET \
               messages = EXCLUDED.messages, voice_minutes = EXCLUDED.voice_minutes, \
               active_members = EXCLUDED.active_members, new_members = EXCLUDED.new_members, \
               leaves = EXCLUDED.leaves, infractions = EXCLUDED.infractions, \
               warns = EXCLUDED.warns, mutes = EXCLUDED.mutes, bans = EXCLUDED.bans"
        );
        sqlx::query(&sql)
            .bind(guild_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn snapshot_hourly(
        &self,
        guild_id: &str,
        track_messages: bool,
    ) -> Result<(), DomainError> {
        let msg_expr = if track_messages {
            "COALESCE((SELECT COUNT(DISTINCT user_id) FROM user_stats WHERE guild_id = $1 AND updated_at >= date_trunc('hour', NOW())), 0)::bigint"
        } else {
            "0::bigint"
        };
        let sql = format!(
            "INSERT INTO hourly_activity (guild_id, day, hour, messages, infractions) \
             SELECT $1, CURRENT_DATE, EXTRACT(HOUR FROM NOW())::smallint, \
               {msg_expr}, \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= date_trunc('hour', NOW()))::integer, 0) \
             ON CONFLICT (guild_id, day, hour) DO UPDATE SET \
               messages = EXCLUDED.messages, infractions = EXCLUDED.infractions"
        );
        sqlx::query(&sql)
            .bind(guild_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn cleanup_daily(&self, guild_id: &str, retention_days: i32) -> Result<(), DomainError> {
        sqlx::query(
            "DELETE FROM daily_activity WHERE guild_id = $1 AND day < CURRENT_DATE - $2::int",
        )
        .bind(guild_id)
        .bind(retention_days)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn cleanup_baseline(
        &self,
        guild_id: &str,
        retention_days: i32,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "DELETE FROM analytics_daily_baseline WHERE guild_id = $1 AND day < CURRENT_DATE - $2::int",
        )
        .bind(guild_id)
        .bind(retention_days)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn cleanup_hourly(&self, guild_id: &str, retention_days: i32) -> Result<(), DomainError> {
        sqlx::query(
            "DELETE FROM hourly_activity WHERE guild_id = $1 AND day < CURRENT_DATE - $2::int",
        )
        .bind(guild_id)
        .bind(retention_days)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
