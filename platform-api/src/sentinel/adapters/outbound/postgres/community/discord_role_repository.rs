use crate::sentinel::adapters::outbound::postgres::pg_ctx;
use async_trait::async_trait;
use sqlx::PgPool;

use platform_core::sentinel::domain::entities::system::discord_role::DiscordRole;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::discord_role_repository::DiscordRoleRepository;

pub struct PgDiscordRoleRepository {
    pool: PgPool,
}

impl PgDiscordRoleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct DiscordRoleRow {
    id: String,
    guild_id: String,
    name: String,
    color: i32,
    position: i32,
    permissions: i64,
    mentionable: bool,
    managed: bool,
    icon: Option<String>,
    member_count: i32,
    synced_at: chrono::DateTime<chrono::Utc>,
}

impl From<DiscordRoleRow> for DiscordRole {
    fn from(r: DiscordRoleRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            name: r.name,
            color: r.color,
            position: r.position,
            permissions: r.permissions,
            mentionable: r.mentionable,
            managed: r.managed,
            icon: r.icon,
            member_count: r.member_count,
            synced_at: r.synced_at,
        }
    }
}

#[async_trait]
impl DiscordRoleRepository for PgDiscordRoleRepository {
    async fn sync_roles(&self, guild_id: &str, roles: Vec<DiscordRole>) -> Result<(), DomainError> {
        // Supprimer les anciens roles du guild puis inserer les nouveaux
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(pg_ctx("Transaction error"))?;

        sqlx::query("DELETE FROM discord_roles WHERE guild_id = $1")
            .bind(guild_id)
            .execute(&mut *tx)
            .await
            .map_err(pg_ctx("Delete roles error"))?;

        for role in &roles {
            sqlx::query(
                "INSERT INTO discord_roles (id, guild_id, name, color, position, permissions, mentionable, managed, icon, member_count, synced_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())"
            )
            .bind(&role.id)
            .bind(guild_id)
            .bind(&role.name)
            .bind(role.color)
            .bind(role.position)
            .bind(role.permissions)
            .bind(role.mentionable)
            .bind(role.managed)
            .bind(&role.icon)
            .bind(role.member_count)
            .execute(&mut *tx)
            .await
            .map_err(pg_ctx("Insert role error"))?;
        }

        tx.commit().await.map_err(pg_ctx("Commit error"))?;

        Ok(())
    }

    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<DiscordRole>, DomainError> {
        let rows = sqlx::query_as::<_, DiscordRoleRow>(
            "SELECT id, guild_id, name, color, position, permissions, mentionable, managed, icon, member_count, synced_at \
             FROM discord_roles WHERE guild_id = $1 ORDER BY position DESC"
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("Query roles error"))?;
        Ok(rows.into_iter().map(DiscordRole::from).collect())
    }

    async fn find_by_id(
        &self,
        guild_id: &str,
        role_id: &str,
    ) -> Result<Option<DiscordRole>, DomainError> {
        let row = sqlx::query_as::<_, DiscordRoleRow>(
            "SELECT id, guild_id, name, color, position, permissions, mentionable, managed, icon, member_count, synced_at \
             FROM discord_roles WHERE guild_id = $1 AND id = $2"
        )
        .bind(guild_id)
        .bind(role_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("Query role error"))?;
        Ok(row.map(DiscordRole::from))
    }
}
