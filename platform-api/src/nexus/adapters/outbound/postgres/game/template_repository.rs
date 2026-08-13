use crate::nexus::adapters::outbound::postgres::pg_ctx;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use platform_core::nexus::domain::entities::game::template::{
    ConfigField, GameTemplate, InitFile, PortProtocol,
};
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::game_template_repository::GameTemplateRepository;

pub struct PgGameTemplateRepository {
    pool: PgPool,
}

impl PgGameTemplateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct TemplateRow {
    id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    image: String,
    category: Option<String>,
    icon: Option<String>,
    accent_color: Option<String>,
    cover_image_url: Option<String>,
    container_port: i32,
    port_protocol: String,
    volume_path: String,
    run_as_root: bool,
    default_memory_mb: i32,
    min_memory_mb: i32,
    max_memory_mb: i32,
    default_env: serde_json::Value,
    config_schema: serde_json::Value,
    supports_rcon: bool,
    supports_mods: bool,
    idle_shutdown_days: i32,
    init_files: serde_json::Value,
    command_template: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<TemplateRow> for GameTemplate {
    type Error = DomainError;
    fn try_from(r: TemplateRow) -> Result<Self, DomainError> {
        let config_schema: Vec<ConfigField> = serde_json::from_value(r.config_schema)
            .map_err(|e| DomainError::Internal(format!("config_schema parse: {e}")))?;
        let init_files: Vec<InitFile> = serde_json::from_value(r.init_files)
            .map_err(|e| DomainError::Internal(format!("init_files parse: {e}")))?;
        let command: Option<Vec<String>> = match r.command_template.as_deref() {
            None | Some("") => None,
            Some(s) => Some(
                serde_json::from_str(s)
                    .map_err(|e| DomainError::Internal(format!("command_template parse: {e}")))?,
            ),
        };
        let port = u16::try_from(r.container_port)
            .map_err(|_| DomainError::Internal("container_port hors range u16".into()))?;
        Ok(GameTemplate {
            id: r.id,
            slug: r.slug,
            name: r.name,
            description: r.description,
            image: r.image,
            category: r.category,
            icon: r.icon,
            accent_color: r.accent_color,
            cover_image_url: r.cover_image_url,
            container_port: port,
            port_protocol: PortProtocol::from_str(&r.port_protocol),
            volume_path: r.volume_path,
            run_as_root: r.run_as_root,
            default_memory_mb: r.default_memory_mb,
            min_memory_mb: r.min_memory_mb,
            max_memory_mb: r.max_memory_mb,
            default_env: r.default_env,
            config_schema,
            supports_rcon: r.supports_rcon,
            supports_mods: r.supports_mods,
            idle_shutdown_days: r.idle_shutdown_days,
            init_files,
            command,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

const SELECT_COLS: &str =
    "id, slug, name, description, image, category, icon, accent_color, cover_image_url, \
     container_port, port_protocol, volume_path, run_as_root, \
     default_memory_mb, min_memory_mb, max_memory_mb, \
     default_env, config_schema, supports_rcon, supports_mods, idle_shutdown_days, \
     init_files, command_template, created_at, updated_at";

#[async_trait]
impl GameTemplateRepository for PgGameTemplateRepository {
    async fn list(&self) -> Result<Vec<GameTemplate>, DomainError> {
        let rows: Vec<TemplateRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLS} FROM game_templates \
             WHERE deleted_at IS NULL ORDER BY name"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("list templates"))?;
        rows.into_iter().map(GameTemplate::try_from).collect()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<GameTemplate>, DomainError> {
        let row: Option<TemplateRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLS} FROM game_templates \
             WHERE id = $1 AND deleted_at IS NULL"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("find template by id"))?;
        row.map(GameTemplate::try_from).transpose()
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<GameTemplate>, DomainError> {
        let row: Option<TemplateRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLS} FROM game_templates \
             WHERE slug = $1 AND deleted_at IS NULL"
        ))
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("find template by slug"))?;
        row.map(GameTemplate::try_from).transpose()
    }
}
