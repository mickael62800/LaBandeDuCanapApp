use crate::sentinel::adapters::outbound::postgres::pg_ctx;
use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::moderation::detection_flags::DetectionFlags;
use platform_core::sentinel::domain::entities::moderation::infraction::Infraction;
use platform_core::sentinel::domain::enums::moderation::action::Action;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::moderation::manage_infractions::InfractionFilters;
use platform_core::sentinel::ports::outbound::moderation::infraction_repository::InfractionRepository;

pub struct PgInfractionRepository {
    pool: PgPool,
}

impl PgInfractionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct InfractionRow {
    id: Uuid,
    guild_id: String,
    channel_id: String,
    user_id: String,
    username: String,
    /// Alias `display_name` issu du LEFT JOIN guild_members. None si l'user
    /// n'est plus / pas dans la guild, ou n'a pas configure de nickname.
    display_name: Option<String>,
    message_id: String,
    content: String,
    flags: serde_json::Value,
    score: f64,
    action: String,
    reason: String,
    duration: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<InfractionRow> for Infraction {
    fn from(row: InfractionRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id.into(),
            channel_id: row.channel_id.into(),
            user_id: row.user_id.into(),
            username: row.username,
            display_name: row.display_name,
            message_id: row.message_id.into(),
            content: row.content,
            flags: serde_json::from_value(row.flags).unwrap_or(DetectionFlags {
                spam: false,
                insult: false,
                profanity: false,
                link: false,
                phishing: false,
            }),
            score: row.score,
            action: Action::from_str_lossy(&row.action),
            reason: row.reason,
            // Negative duration (DB corruption / bogus migration) → None au lieu
            // de wrap silencieusement sur u64::MAX via `as u64`.
            duration: row.duration.and_then(|d| u64::try_from(d).ok()),
            created_at: row.created_at,
        }
    }
}

/// SELECT enrichi : ajoute `gm.display_name AS display_name` via LEFT JOIN
/// `guild_members`. Utilise dans toutes les queries qui retournent une
/// `Infraction` au front.
const INFRACTION_SELECT: &str = "SELECT i.id, i.guild_id, i.channel_id, i.user_id, i.username, \
    gm.display_name AS display_name, i.message_id, i.content, i.flags, i.score, \
    i.action, i.reason, i.duration, i.created_at \
    FROM infractions i \
    LEFT JOIN guild_members gm ON gm.guild_id = i.guild_id AND gm.user_id = i.user_id";

#[async_trait]
impl InfractionRepository for PgInfractionRepository {
    async fn save(&self, infraction: &Infraction) -> Result<(), DomainError> {
        let flags_json = serde_json::to_value(&infraction.flags)
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO infractions (id, guild_id, channel_id, user_id, username, message_id, content, flags, score, action, reason, duration, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(infraction.id)
        .bind(infraction.guild_id.as_str())
        .bind(infraction.channel_id.as_str())
        .bind(infraction.user_id.as_str())
        .bind(&infraction.username)
        .bind(infraction.message_id.as_str())
        .bind(&infraction.content)
        .bind(flags_json)
        .bind(infraction.score)
        .bind(infraction.action.as_str())
        .bind(&infraction.reason)
        .bind(infraction.duration.map(|d| d as i64))
        .bind(infraction.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn find_by_guild(
        &self,
        guild_id: &str,
        filters: &InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError> {
        let mut query = format!("{INFRACTION_SELECT} WHERE i.guild_id = $1");
        let mut param_idx = 2u32;

        if filters.user_id.is_some() {
            query.push_str(&format!(" AND i.user_id = ${}", param_idx));
            param_idx += 1;
        }
        if filters.action.is_some() {
            query.push_str(&format!(" AND i.action = ${}", param_idx));
            param_idx += 1;
        }

        query.push_str(&format!(
            " ORDER BY i.created_at DESC LIMIT ${} OFFSET ${}",
            param_idx,
            param_idx + 1
        ));

        let mut q = sqlx::query_as::<_, InfractionRow>(&query).bind(guild_id);

        if let Some(ref user_id) = filters.user_id {
            q = q.bind(user_id);
        }
        if let Some(ref action) = filters.action {
            q = q.bind(action);
        }

        q = q.bind(filters.limit).bind(filters.offset);

        let rows = q.fetch_all(&self.pool).await.map_err(pg_err)?;

        Ok(rows.into_iter().map(Infraction::from).collect())
    }

    async fn find_all(&self, limit: i64, offset: i64) -> Result<Vec<Infraction>, DomainError> {
        let sql = format!("{INFRACTION_SELECT} ORDER BY i.created_at DESC LIMIT $1 OFFSET $2");
        let rows = sqlx::query_as::<_, InfractionRow>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(rows.into_iter().map(Infraction::from).collect())
    }

    async fn count_today(&self) -> Result<u64, DomainError> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM infractions WHERE created_at >= CURRENT_DATE")
                .fetch_one(&self.pool)
                .await
                .map_err(pg_err)?;

        Ok(row.0 as u64)
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Infraction>, DomainError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| DomainError::NotFound(format!("ID invalide: {id}")))?;

        let sql = format!("{INFRACTION_SELECT} WHERE i.id = $1");
        let row = sqlx::query_as::<_, InfractionRow>(&sql)
            .bind(uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(row.map(Infraction::from))
    }

    async fn delete_by_id(&self, id: &str) -> Result<bool, DomainError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| DomainError::NotFound(format!("ID invalide: {id}")))?;

        let result = sqlx::query("DELETE FROM infractions WHERE id = $1")
            .bind(uuid)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(result.rows_affected() > 0)
    }

    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM infractions WHERE guild_id = $1 AND created_at < NOW() - make_interval(days => $2)")
            .bind(guild_id)
            .bind(days)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("delete_infractions_older"))?;
        Ok(result.rows_affected())
    }

    async fn count_by_action_for_user(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<(String, u64)>, DomainError> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT action, COUNT(*) FROM infractions \
             WHERE guild_id = $1 AND user_id = $2 GROUP BY action",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("count_infractions_by_action"))?;
        Ok(rows
            .into_iter()
            .map(|(a, n)| (a, n.max(0) as u64))
            .collect())
    }
}

#[cfg(test)]
#[path = "tests/infraction_repository.rs"]
mod tests;
