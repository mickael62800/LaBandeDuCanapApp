use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::daily_activity::DailyActivity;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::daily_activity_repository::DailyActivityRepository;

pub struct PgDailyActivityRepository {
    pool: PgPool,
}

impl PgDailyActivityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct DailyActivityRow {
    id: Uuid,
    guild_id: String,
    day: NaiveDate,
    messages: i64,
    voice_minutes: i64,
    active_members: i32,
    new_members: i32,
    leaves: i32,
    infractions: i32,
    warns: i32,
    mutes: i32,
    bans: i32,
}

impl From<DailyActivityRow> for DailyActivity {
    fn from(r: DailyActivityRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            day: r.day,
            messages: r.messages,
            voice_minutes: r.voice_minutes,
            active_members: r.active_members,
            new_members: r.new_members,
            leaves: r.leaves,
            infractions: r.infractions,
            warns: r.warns,
            mutes: r.mutes,
            bans: r.bans,
        }
    }
}

#[async_trait]
impl DailyActivityRepository for PgDailyActivityRepository {
    async fn get_activity(
        &self,
        guild_id: Option<&str>,
        days: i32,
    ) -> Result<Vec<DailyActivity>, DomainError> {
        let query = if guild_id.is_some() {
            r#"SELECT * FROM daily_activity
               WHERE guild_id = $1 AND day >= CURRENT_DATE - $2::integer
               ORDER BY day ASC"#
        } else {
            r#"SELECT '00000000-0000-0000-0000-000000000000'::uuid as id, '' as guild_id, day,
                      SUM(messages)::bigint as messages,
                      SUM(voice_minutes)::bigint as voice_minutes,
                      SUM(active_members)::integer as active_members,
                      SUM(new_members)::integer as new_members,
                      SUM(leaves)::integer as leaves,
                      SUM(infractions)::integer as infractions,
                      SUM(warns)::integer as warns,
                      SUM(mutes)::integer as mutes,
                      SUM(bans)::integer as bans
               FROM daily_activity
               WHERE day >= CURRENT_DATE - $1::integer
               GROUP BY day
               ORDER BY day ASC"#
        };

        let rows = if let Some(gid) = guild_id {
            sqlx::query_as::<_, DailyActivityRow>(query)
                .bind(gid)
                .bind(days)
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query_as::<_, DailyActivityRow>(query)
                .bind(days)
                .fetch_all(&self.pool)
                .await
        };

        let rows = rows.map_err(pg_err)?;
        Ok(rows.into_iter().map(DailyActivity::from).collect())
    }

    async fn record_daily_snapshot(&self, guild_id: &str) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO daily_activity (guild_id, day, messages, voice_minutes, active_members, infractions, warns, mutes, bans)
               SELECT
                 $1,
                 CURRENT_DATE,
                 COALESCE((SELECT SUM(message_count) FROM user_stats WHERE guild_id = $1), 0),
                 COALESCE((SELECT SUM(voice_seconds) / 60 FROM user_stats WHERE guild_id = $1), 0),
                 COALESCE((SELECT COUNT(DISTINCT user_id) FROM user_stats WHERE guild_id = $1 AND updated_at >= CURRENT_DATE), 0)::integer,
                 COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= CURRENT_DATE)::integer, 0),
                 COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= CURRENT_DATE AND action = 'warn')::integer, 0),
                 COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= CURRENT_DATE AND action = 'mute')::integer, 0),
                 COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= CURRENT_DATE AND action = 'ban')::integer, 0)
               ON CONFLICT (guild_id, day) DO UPDATE SET
                 messages = EXCLUDED.messages,
                 voice_minutes = EXCLUDED.voice_minutes,
                 active_members = EXCLUDED.active_members,
                 infractions = EXCLUDED.infractions,
                 warns = EXCLUDED.warns,
                 mutes = EXCLUDED.mutes,
                 bans = EXCLUDED.bans"#,
        )
        .bind(guild_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }
}
