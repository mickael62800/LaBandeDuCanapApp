use async_trait::async_trait;
use sqlx::PgPool;

use super::super::pg_err;
use platform_core::sentinel::domain::entities::moderation::modstats::ModeratorBreakdown;
use platform_core::sentinel::domain::entities::moderation::modstats::ModstatsTrendDay;
use platform_core::sentinel::ports::outbound::audit::modstats_repository::ModeratorStat;
use platform_core::sentinel::ports::outbound::audit::modstats_repository::ModstatsRepository;

pub struct PgModstatsRepository {
    pool: PgPool,
}

impl PgModstatsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    moderator_id: String,
    moderator_name: String,
    action_count: i64,
}

#[async_trait]
impl ModstatsRepository for PgModstatsRepository {
    async fn top_moderators(
        &self,
        guild_id: &str,
        days: i32,
        limit: i64,
    ) -> Result<Vec<ModeratorStat>, platform_core::sentinel::domain::errors::DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT actor_id AS moderator_id, \
                    COALESCE(MAX(actor_name), actor_id) AS moderator_name, \
                    COUNT(*) AS action_count \
             FROM audit_logs \
             WHERE guild_id = $1 AND event_type LIKE 'mod_%' \
               AND event_type NOT IN ('mod_unban', 'mod_unmute') \
               AND actor_id IS NOT NULL \
               AND created_at >= NOW() - make_interval(days => $2) \
             GROUP BY actor_id \
             ORDER BY action_count DESC \
             LIMIT $3",
        )
        .bind(guild_id)
        .bind(days)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| ModeratorStat {
                moderator_id: r.moderator_id,
                moderator_name: r.moderator_name,
                action_count: r.action_count,
            })
            .collect())
    }

    async fn breakdown(
        &self,
        guild_id: &str,
        days: i32,
        limit: i64,
    ) -> Result<Vec<ModeratorBreakdown>, platform_core::sentinel::domain::errors::DomainError> {
        #[derive(sqlx::FromRow)]
        struct BreakdownRow {
            moderator_id: String,
            moderator_name: String,
            total: i64,
            warns: i64,
            mutes: i64,
            bans: i64,
            kicks: i64,
        }

        // Les statistiques partagent la source de verite `audit_logs`.
        // `days` est un i32 clampe cote use case (interpolation safe).
        let sql = format!(
            "SELECT \
                actor_id AS moderator_id, \
                MAX(actor_name) AS moderator_name, \
                COUNT(*) AS total, \
                COUNT(*) FILTER (WHERE event_type = 'mod_warn') AS warns, \
                COUNT(*) FILTER (WHERE event_type IN ('mod_mute_temp','mod_mute_permanent','mod_mute')) AS mutes, \
                COUNT(*) FILTER (WHERE event_type IN ('mod_ban_temp','mod_ban_permanent','mod_ban')) AS bans, \
                COUNT(*) FILTER (WHERE event_type = 'mod_kick') AS kicks \
             FROM audit_logs \
             WHERE guild_id = $1 \
               AND event_type LIKE 'mod_%' \
               AND event_type NOT IN ('mod_unban','mod_unmute') \
               AND actor_id IS NOT NULL \
               AND created_at >= NOW() - INTERVAL '{days} days' \
             GROUP BY actor_id \
             ORDER BY total DESC \
             LIMIT {limit}"
        );
        let rows: Vec<BreakdownRow> = sqlx::query_as::<_, BreakdownRow>(&sql)
            .bind(guild_id)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| ModeratorBreakdown {
                moderator_id: r.moderator_id,
                moderator_name: r.moderator_name,
                total: r.total,
                warns: r.warns,
                mutes: r.mutes,
                bans: r.bans,
                kicks: r.kicks,
            })
            .collect())
    }

    async fn daily_trend(
        &self,
        guild_id: &str,
        days: i32,
    ) -> Result<Vec<ModstatsTrendDay>, platform_core::sentinel::domain::errors::DomainError> {
        #[derive(sqlx::FromRow)]
        struct TrendRow {
            day: chrono::NaiveDate,
            warns: i64,
            mutes: i64,
            bans: i64,
            kicks: i64,
        }

        // generate_series garantit que tous les jours apparaissent meme s'il
        // n'y a eu aucune action ce jour-la (sinon la courbe a des trous).
        let sql = format!(
            "SELECT \
                d::date AS day, \
                COALESCE(SUM(CASE WHEN a.event_type = 'mod_warn' THEN 1 ELSE 0 END), 0) AS warns, \
                COALESCE(SUM(CASE WHEN a.event_type IN ('mod_mute_temp','mod_mute_permanent','mod_mute') THEN 1 ELSE 0 END), 0) AS mutes, \
                COALESCE(SUM(CASE WHEN a.event_type IN ('mod_ban_temp','mod_ban_permanent','mod_ban') THEN 1 ELSE 0 END), 0) AS bans, \
                COALESCE(SUM(CASE WHEN a.event_type = 'mod_kick' THEN 1 ELSE 0 END), 0) AS kicks \
             FROM generate_series( \
                     (CURRENT_DATE - INTERVAL '{days} days')::date, \
                     CURRENT_DATE, \
                     INTERVAL '1 day' \
                  ) AS d \
             LEFT JOIN audit_logs a \
                 ON a.guild_id = $1 \
                 AND a.event_type LIKE 'mod_%' \
                 AND a.event_type NOT IN ('mod_unban','mod_unmute') \
                 AND a.actor_id IS NOT NULL \
                 AND a.created_at::date = d::date \
             GROUP BY d \
             ORDER BY d ASC"
        );
        let rows: Vec<TrendRow> = sqlx::query_as::<_, TrendRow>(&sql)
            .bind(guild_id)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| ModstatsTrendDay {
                day: r.day,
                warns: r.warns,
                mutes: r.mutes,
                bans: r.bans,
                kicks: r.kicks,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "./migrations/sentinel")]
    async fn top_moderators_reads_mod_events_from_audit_logs(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO audit_logs \
                 (id, guild_id, event_type, actor_id, actor_name, target_id, details) \
             VALUES (gen_random_uuid(), 'guild-1', 'mod_warn', 'moderator-1', \
                     'Moderator', 'target-1', '{\"reason\":\"test\"}'::jsonb)",
        )
        .execute(&pool)
        .await?;

        let stats = PgModstatsRepository::new(pool)
            .top_moderators("guild-1", 30, 10)
            .await
            .unwrap();

        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].moderator_id, "moderator-1");
        assert_eq!(stats[0].action_count, 1);
        Ok(())
    }
}
