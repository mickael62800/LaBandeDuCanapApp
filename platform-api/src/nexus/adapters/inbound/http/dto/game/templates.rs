use serde::Serialize;
use uuid::Uuid;

use platform_core::nexus::domain::entities::game::template::{ConfigField, GameTemplate};

/// Représentation API d'un template installable.
///
/// `config_schema` décrit les clés que le client peut envoyer. Une clé absente
/// du schéma doit être refusée par la validation du domaine.
#[derive(Debug, Serialize)]
pub struct GameTemplateDto {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub icon: Option<String>,
    pub accent_color: Option<String>,
    pub cover_image_url: Option<String>,
    pub container_port: u16,
    pub port_protocol: String,
    pub default_memory_mb: i32,
    pub min_memory_mb: i32,
    pub max_memory_mb: i32,
    pub config_schema: Vec<ConfigField>,
    pub supports_rcon: bool,
    pub supports_mods: bool,
    pub idle_shutdown_days: i32,
}

impl From<GameTemplate> for GameTemplateDto {
    fn from(t: GameTemplate) -> Self {
        Self {
            id: t.id,
            slug: t.slug,
            name: t.name,
            description: t.description,
            category: t.category,
            icon: t.icon,
            accent_color: t.accent_color,
            cover_image_url: t.cover_image_url,
            container_port: t.container_port,
            port_protocol: t.port_protocol.as_str().to_string(),
            default_memory_mb: t.default_memory_mb,
            min_memory_mb: t.min_memory_mb,
            max_memory_mb: t.max_memory_mb,
            config_schema: t.config_schema,
            supports_rcon: t.supports_rcon,
            supports_mods: t.supports_mods,
            idle_shutdown_days: t.idle_shutdown_days,
        }
    }
}
