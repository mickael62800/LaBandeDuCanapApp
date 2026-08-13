use crate::nexus::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use platform_core::nexus::domain::entities::system::bot_config::BotDefinition;
use platform_core::nexus::domain::entities::system::bot_config::BotGuildConfig;
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository;

pub struct PgBotConfigRepository {
    pool: PgPool,
}

impl PgBotConfigRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ConfigRow {
    id: Uuid,
    guild_id: String,
    bot_name: String,
    config_key: String,
    config_value: String,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<ConfigRow> for BotGuildConfig {
    fn from(row: ConfigRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id.into(),
            bot_name: row.bot_name,
            config_key: row.config_key,
            config_value: row.config_value,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct DefinitionRow {
    bot_name: String,
    display_name: String,
    description: String,
    config_schema: serde_json::Value,
}

impl From<DefinitionRow> for BotDefinition {
    fn from(row: DefinitionRow) -> Self {
        Self {
            bot_name: row.bot_name,
            display_name: row.display_name,
            description: row.description,
            config_schema: row.config_schema,
        }
    }
}

#[async_trait]
impl BotConfigRepository for PgBotConfigRepository {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
        let rows = sqlx::query_as::<_, DefinitionRow>(
            "SELECT bot_name, display_name, description, config_schema FROM bot_definitions ORDER BY display_name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(BotDefinition::from).collect())
    }

    async fn get_config(
        &self,
        guild_id: &str,
        bot_name: &str,
    ) -> Result<Vec<BotGuildConfig>, DomainError> {
        let rows = sqlx::query_as::<_, ConfigRow>(
            "SELECT * FROM bot_guild_config WHERE guild_id = $1 AND bot_name = $2 ORDER BY config_key",
        )
        .bind(guild_id)
        .bind(bot_name)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(BotGuildConfig::from).collect())
    }

    async fn get_all_config(&self, guild_id: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        let rows = sqlx::query_as::<_, ConfigRow>(
            "SELECT * FROM bot_guild_config WHERE guild_id = $1 ORDER BY bot_name, config_key",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(BotGuildConfig::from).collect())
    }

    async fn set_config(
        &self,
        guild_id: &str,
        bot_name: &str,
        key: &str,
        value: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO bot_guild_config (id, guild_id, bot_name, config_key, config_value, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (guild_id, bot_name, config_key) DO UPDATE SET
                config_value = EXCLUDED.config_value,
                updated_at = NOW()
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(guild_id)
        .bind(bot_name)
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn delete_config(
        &self,
        guild_id: &str,
        bot_name: &str,
        key: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "DELETE FROM bot_guild_config WHERE guild_id = $1 AND bot_name = $2 AND config_key = $3",
        )
        .bind(guild_id)
        .bind(bot_name)
        .bind(key)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }
}
