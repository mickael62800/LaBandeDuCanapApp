use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::nexus::adapters::outbound::postgres::pg_err;
use platform_core::nexus::domain::entities::game::session::{
    GameSessionRegistration, GameTemplateSettings,
};
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::game_session_repository::{
    GameSessionRegistrationRepository, GameTemplateSettingsRepository,
};

// ── Reglages par (guild, template) ──

#[derive(FromRow)]
struct SettingsRow {
    guild_id: String,
    template_slug: String,
    discord_role_id: Option<String>,
}

impl From<SettingsRow> for GameTemplateSettings {
    fn from(r: SettingsRow) -> Self {
        Self {
            guild_id: r.guild_id,
            template_slug: r.template_slug,
            discord_role_id: r.discord_role_id,
        }
    }
}

pub struct PgGameTemplateSettingsRepository {
    pool: PgPool,
}

impl PgGameTemplateSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GameTemplateSettingsRepository for PgGameTemplateSettingsRepository {
    async fn get(
        &self,
        guild_id: &str,
        template_slug: &str,
    ) -> Result<Option<GameTemplateSettings>, DomainError> {
        let row = sqlx::query_as::<_, SettingsRow>(
            "SELECT guild_id, template_slug, discord_role_id FROM game_template_settings \
             WHERE guild_id = $1 AND template_slug = $2",
        )
        .bind(guild_id)
        .bind(template_slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(GameTemplateSettings::from))
    }

    async fn list(&self, guild_id: &str) -> Result<Vec<GameTemplateSettings>, DomainError> {
        let rows = sqlx::query_as::<_, SettingsRow>(
            "SELECT guild_id, template_slug, discord_role_id FROM game_template_settings \
             WHERE guild_id = $1",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(GameTemplateSettings::from).collect())
    }

    async fn set_role(
        &self,
        guild_id: &str,
        template_slug: &str,
        discord_role_id: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO game_template_settings (guild_id, template_slug, discord_role_id) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (guild_id, template_slug) DO UPDATE SET \
               discord_role_id = EXCLUDED.discord_role_id, updated_at = NOW()",
        )
        .bind(guild_id)
        .bind(template_slug)
        .bind(discord_role_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}

// ── Inscriptions a une session ──

#[derive(FromRow)]
struct RegRow {
    id: Uuid,
    server_id: Uuid,
    user_id: String,
    registered_at: DateTime<Utc>,
}

impl From<RegRow> for GameSessionRegistration {
    fn from(r: RegRow) -> Self {
        Self {
            id: r.id,
            server_id: r.server_id,
            user_id: r.user_id,
            registered_at: r.registered_at,
        }
    }
}

pub struct PgGameSessionRegistrationRepository {
    pool: PgPool,
}

impl PgGameSessionRegistrationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GameSessionRegistrationRepository for PgGameSessionRegistrationRepository {
    async fn register(&self, server_id: Uuid, user_id: &str) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO game_session_registrations (server_id, user_id) VALUES ($1, $2) \
             ON CONFLICT (server_id, user_id) DO NOTHING",
        )
        .bind(server_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn unregister(&self, server_id: Uuid, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM game_session_registrations WHERE server_id = $1 AND user_id = $2")
            .bind(server_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn list(&self, server_id: Uuid) -> Result<Vec<GameSessionRegistration>, DomainError> {
        let rows = sqlx::query_as::<_, RegRow>(
            "SELECT id, server_id, user_id, registered_at FROM game_session_registrations \
             WHERE server_id = $1 ORDER BY registered_at ASC",
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(GameSessionRegistration::from)
            .collect())
    }
}
