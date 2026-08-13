use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;

use platform_core::sentinel::domain::entities::audit::watched_user::classify_risk_level;
use platform_core::sentinel::domain::entities::audit::watched_user::WatchedUser;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::audit::watched_user_repository::WatchedUserRepository;

pub struct PgWatchedUserRepository {
    pool: PgPool,
}

impl PgWatchedUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct WatchedUserRow {
    user_id: String,
    username: String,
    guild_id: String,
    guild_name: String,
    total_warns: i64,
    total_mutes: i64,
    total_bans: i64,
    last_incident_at: Option<chrono::DateTime<chrono::Utc>>,
    security_events_count: i64,
    first_seen_at: chrono::DateTime<chrono::Utc>,
}

impl From<WatchedUserRow> for WatchedUser {
    fn from(row: WatchedUserRow) -> Self {
        // La regle de classification de risque est en `domain/entities/watched_user.rs`.
        // Cet adapter se contente de mapper row → entity + appel de la fn pure.
        let risk_level =
            classify_risk_level(row.total_warns, row.total_mutes, row.total_bans).to_string();

        Self {
            user_id: row.user_id.into(),
            username: row.username,
            guild_id: row.guild_id.into(),
            guild_name: row.guild_name,
            risk_level,
            total_warns: row.total_warns,
            total_mutes: row.total_mutes,
            total_bans: row.total_bans,
            last_incident_at: row.last_incident_at,
            security_events_count: row.security_events_count,
            first_seen_at: row.first_seen_at,
        }
    }
}

#[async_trait]
impl WatchedUserRepository for PgWatchedUserRepository {
    async fn find_watched_users(
        &self,
        guild_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WatchedUser>, DomainError> {
        // Phase X — surveillance purement MANUELLE : avant, la requete
        // faisait un UNION entre tous les users avec infractions (auto) et
        // les users dans manual_watched_users. Consequence : impossible de
        // retirer un user avec des infractions (il revenait via la branche
        // auto), et impossible d'ajouter un user deja auto-tracke (il etait
        // deja "watched"). On ne retourne maintenant QUE les entrees
        // manual_watched_users, enrichies avec leurs stats d'infractions.
        // Les compteurs d'infractions (warn/mute/ban + dernier incident) sont
        // calcules en UN seul scan par user via un LEFT JOIN LATERAL + COUNT
        // FILTER, au lieu de 4 sous-requetes correlees (1 scan de `infractions`
        // au lieu de 4). L'agregat sans GROUP BY renvoie exactement une ligne
        // par `mw`, donc pas de duplication.
        let query = r#"
            SELECT
                mw.user_id,
                mw.username,
                mw.guild_id,
                COALESCE(g.name, mw.guild_id) AS guild_name,
                COALESCE(inf.total_warns, 0) AS total_warns,
                COALESCE(inf.total_mutes, 0) AS total_mutes,
                COALESCE(inf.total_bans, 0) AS total_bans,
                inf.last_incident_at AS last_incident_at,
                COALESCE((
                    -- Phase 4 : `security_events` n'est plus ecrite (save() est
                    -- un no-op). Les events vivent dans `audit_logs`
                    -- (event_type 'security_%'), les user_ids dans
                    -- details->'user_ids' (meme shape que la lecture de
                    -- PgSecurityEventRepository::find_by_guild).
                    SELECT COUNT(*)::bigint
                    FROM audit_logs se,
                         jsonb_array_elements_text(se.details->'user_ids') AS u(user_id)
                    WHERE se.event_type LIKE 'security_%'
                      AND se.guild_id = mw.guild_id AND u.user_id = mw.user_id
                ), 0) AS security_events_count,
                mw.created_at AS first_seen_at
            FROM manual_watched_users mw
            LEFT JOIN guilds g ON g.guild_id = mw.guild_id
            LEFT JOIN LATERAL (
                SELECT
                    COUNT(*) FILTER (WHERE i.action = 'warn')::bigint AS total_warns,
                    COUNT(*) FILTER (WHERE i.action = 'mute')::bigint AS total_mutes,
                    COUNT(*) FILTER (WHERE i.action = 'ban')::bigint  AS total_bans,
                    MAX(i.created_at) AS last_incident_at
                FROM infractions i
                WHERE i.guild_id = mw.guild_id AND i.user_id = mw.user_id
            ) inf ON true
            WHERE ($1::text IS NULL OR mw.guild_id = $1)
            ORDER BY mw.created_at DESC
            LIMIT $2 OFFSET $3
        "#;

        let rows = sqlx::query_as::<_, WatchedUserRow>(query)
            .bind(guild_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(rows.into_iter().map(WatchedUser::from).collect())
    }

    async fn add_manual_watch(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        reason: &str,
        added_by: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO manual_watched_users (guild_id, user_id, username, reason, added_by)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (guild_id, user_id) DO UPDATE SET
                username = EXCLUDED.username,
                reason = EXCLUDED.reason
            "#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(username)
        .bind(reason)
        .bind(added_by)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn remove_manual_watch(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM manual_watched_users WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/watched_user_repository.rs"]
mod tests;
