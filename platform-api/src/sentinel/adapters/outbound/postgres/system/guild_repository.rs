use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;

use platform_core::sentinel::domain::entities::system::guild::Guild;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::system::guild_repository::GuildRepository;

pub struct PgGuildRepository {
    pool: PgPool,
}

impl PgGuildRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct GuildRow {
    guild_id: String,
    name: String,
    icon: Option<String>,
    member_count: i32,
    registered_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<GuildRow> for Guild {
    fn from(row: GuildRow) -> Self {
        Self {
            guild_id: row.guild_id.into(),
            name: row.name,
            icon: row.icon,
            member_count: row.member_count,
            registered_at: row.registered_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl GuildRepository for PgGuildRepository {
    async fn upsert(&self, guild: &Guild) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO guilds (guild_id, name, icon, member_count, registered_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (guild_id) DO UPDATE SET
                name = EXCLUDED.name,
                icon = EXCLUDED.icon,
                member_count = EXCLUDED.member_count,
                updated_at = NOW()
            "#,
        )
        .bind(guild.guild_id.as_str())
        .bind(&guild.name)
        .bind(&guild.icon)
        .bind(guild.member_count)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn find_all(&self) -> Result<Vec<Guild>, DomainError> {
        let rows = sqlx::query_as::<_, GuildRow>("SELECT * FROM guilds ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(rows.into_iter().map(Guild::from).collect())
    }

    async fn find_by_id(&self, guild_id: &str) -> Result<Option<Guild>, DomainError> {
        let row = sqlx::query_as::<_, GuildRow>("SELECT * FROM guilds WHERE guild_id = $1")
            .bind(guild_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(row.map(Guild::from))
    }

    async fn delete(&self, guild_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM guilds WHERE guild_id = $1")
            .bind(guild_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn delete_absent(&self, keep_ids: &[String]) -> Result<u64, DomainError> {
        // Garde de securite : une liste vide signifierait "le bot n'est dans
        // aucune guild", ce qui est presque toujours un faux signal (gateway
        // pas encore pret). On refuse de tout supprimer dans ce cas.
        if keep_ids.is_empty() {
            return Ok(0);
        }

        let result = sqlx::query("DELETE FROM guilds WHERE guild_id <> ALL($1)")
            .bind(keep_ids)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(result.rows_affected())
    }
}
