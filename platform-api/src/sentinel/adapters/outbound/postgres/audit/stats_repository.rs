use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::audit::user_stats::UserStats;
use platform_core::sentinel::domain::entities::audit::user_stats::VoiceSessionStats;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::audit::stats_repository::StatsRepository;

pub struct PgStatsRepository {
    pool: PgPool,
}

impl PgStatsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct StatsRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    username: String,
    message_count: i64,
    voice_seconds: i64,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<StatsRow> for UserStats {
    fn from(row: StatsRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id.into(),
            user_id: row.user_id.into(),
            username: row.username,
            message_count: row.message_count as u64,
            voice_seconds: row.voice_seconds as u64,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl StatsRepository for PgStatsRepository {
    async fn upsert(&self, stats: &UserStats) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO user_stats (id, guild_id, user_id, username, message_count, voice_seconds, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (guild_id, user_id) DO UPDATE SET
                username = EXCLUDED.username,
                message_count = EXCLUDED.message_count,
                voice_seconds = EXCLUDED.voice_seconds,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(stats.id)
        .bind(stats.guild_id.as_str())
        .bind(stats.user_id.as_str())
        .bind(&stats.username)
        .bind(stats.message_count as i64)
        .bind(stats.voice_seconds as i64)
        .bind(stats.updated_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn find_by_user(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserStats>, DomainError> {
        let row = sqlx::query_as::<_, StatsRow>(
            "SELECT * FROM user_stats WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.map(UserStats::from))
    }

    async fn find_by_guild(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<UserStats>, DomainError> {
        let rows = sqlx::query_as::<_, StatsRow>(
            "SELECT * FROM user_stats WHERE guild_id = $1 ORDER BY message_count DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(UserStats::from).collect())
    }

    async fn increment_messages(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        count: u64,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO user_stats (id, guild_id, user_id, username, message_count, voice_seconds, updated_at)
            VALUES ($1, $2, $3, $4, $5, 0, NOW())
            ON CONFLICT (guild_id, user_id) DO UPDATE SET
                username = EXCLUDED.username,
                message_count = user_stats.message_count + EXCLUDED.message_count,
                updated_at = NOW()
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(guild_id)
        .bind(user_id)
        .bind(username)
        .bind(count as i64)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn add_voice_seconds(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        seconds: u64,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO user_stats (id, guild_id, user_id, username, message_count, voice_seconds, updated_at)
            VALUES ($1, $2, $3, $4, 0, $5, NOW())
            ON CONFLICT (guild_id, user_id) DO UPDATE SET
                username = EXCLUDED.username,
                voice_seconds = user_stats.voice_seconds + EXCLUDED.voice_seconds,
                updated_at = NOW()
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(guild_id)
        .bind(user_id)
        .bind(username)
        .bind(seconds as i64)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn count_distinct_guilds(&self) -> Result<u64, DomainError> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(DISTINCT guild_id) FROM user_stats")
            .fetch_one(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(row.0 as u64)
    }

    async fn count_distinct_users(&self) -> Result<u64, DomainError> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(DISTINCT user_id) FROM user_stats")
            .fetch_one(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(row.0 as u64)
    }

    async fn save_voice_session(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        channel_id: &str,
        channel_name: &str,
        duration_secs: u64,
    ) -> Result<(), DomainError> {
        let now = chrono::Utc::now();
        let started_at = now - chrono::Duration::seconds(duration_secs as i64);

        sqlx::query(
            r#"
            INSERT INTO voice_sessions (guild_id, user_id, username, channel_id, channel_name, duration_secs, started_at, ended_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(username)
        .bind(channel_id)
        .bind(channel_name)
        .bind(duration_secs as i64)
        .bind(started_at)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn get_guild_voice_stats(
        &self,
        guild_id: &str,
        days: u32,
        limit: u32,
    ) -> Result<Vec<VoiceSessionStats>, DomainError> {
        let since = chrono::Utc::now() - chrono::Duration::days(days as i64);

        #[derive(sqlx::FromRow)]
        struct Row {
            channel_id: String,
            channel_name: String,
            is_temporary: bool,
            total_sessions: i64,
            total_duration_secs: i64,
            unique_users: i64,
            avg_duration_secs: i64,
            last_activity: Option<chrono::DateTime<chrono::Utc>>,
        }

        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT
                vs.channel_id,
                (array_agg(vs.channel_name ORDER BY vs.ended_at DESC))[1] as channel_name,
                BOOL_OR(vc.id IS NOT NULL AND vc.channel_status = 'open') as is_temporary,
                COUNT(*)::BIGINT as total_sessions,
                COALESCE(SUM(vs.duration_secs), 0)::BIGINT as total_duration_secs,
                COUNT(DISTINCT vs.user_id)::BIGINT as unique_users,
                COALESCE(AVG(vs.duration_secs), 0)::BIGINT as avg_duration_secs,
                MAX(vs.ended_at) as last_activity
            FROM voice_sessions vs
            LEFT JOIN voice_channels vc ON vs.channel_id = vc.channel_id AND vc.channel_status = 'open'
            WHERE vs.guild_id = $1 AND vs.started_at > $2
            GROUP BY vs.channel_id
            ORDER BY total_duration_secs DESC
            LIMIT $3
            "#,
        )
        .bind(guild_id)
        .bind(since)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| VoiceSessionStats {
                channel_id: r.channel_id.into(),
                channel_name: r.channel_name,
                is_temporary: r.is_temporary,
                total_sessions: r.total_sessions,
                total_duration_secs: r.total_duration_secs,
                unique_users: r.unique_users,
                avg_duration_secs: r.avg_duration_secs,
                last_activity: r.last_activity,
            })
            .collect())
    }

    async fn count_unique_voice_users(
        &self,
        guild_id: &str,
        days: u32,
    ) -> Result<i64, DomainError> {
        let since = chrono::Utc::now() - chrono::Duration::days(days as i64);

        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT user_id)::BIGINT FROM voice_sessions WHERE guild_id = $1 AND started_at > $2",
        )
        .bind(guild_id)
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.0)
    }
}
