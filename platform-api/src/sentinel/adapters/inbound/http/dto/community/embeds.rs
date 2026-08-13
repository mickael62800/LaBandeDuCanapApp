use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::embed::{Embed, EmbedField};
use platform_core::sentinel::ports::inbound::community::manage_embeds::EmbedInput;

#[derive(Debug, Deserialize)]
pub struct EmbedInputDto {
    pub name: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub author_name: String,
    #[serde(default)]
    pub author_icon_url: String,
    #[serde(default)]
    pub author_url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub title_url: String,
    #[serde(default)]
    pub description: String,
    pub color: Option<i32>,
    #[serde(default)]
    pub image_url: String,
    #[serde(default)]
    pub thumbnail_url: String,
    #[serde(default)]
    pub footer_text: String,
    #[serde(default)]
    pub footer_icon_url: String,
    #[serde(default)]
    pub show_timestamp: bool,
    #[serde(default)]
    pub fields: Vec<EmbedField>,
}

impl From<EmbedInputDto> for EmbedInput {
    fn from(d: EmbedInputDto) -> Self {
        Self {
            name: d.name,
            content: d.content,
            author_name: d.author_name,
            author_icon_url: d.author_icon_url,
            author_url: d.author_url,
            title: d.title,
            title_url: d.title_url,
            description: d.description,
            color: d.color,
            image_url: d.image_url,
            thumbnail_url: d.thumbnail_url,
            footer_text: d.footer_text,
            footer_icon_url: d.footer_icon_url,
            show_timestamp: d.show_timestamp,
            fields: d.fields,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PostEmbedDto {
    pub channel_id: String,
}

#[derive(Debug, Deserialize)]
pub struct EmbedPostedDto {
    pub channel_id: String,
    pub message_id: String,
}

#[derive(Debug, Serialize)]
pub struct EmbedDto {
    pub id: Uuid,
    pub guild_id: String,
    pub name: String,
    pub content: String,
    pub author_name: String,
    pub author_icon_url: String,
    pub author_url: String,
    pub title: String,
    pub title_url: String,
    pub description: String,
    pub color: Option<i32>,
    pub image_url: String,
    pub thumbnail_url: String,
    pub footer_text: String,
    pub footer_icon_url: String,
    pub show_timestamp: bool,
    pub fields: Vec<EmbedField>,
    pub last_channel_id: Option<String>,
    pub last_message_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Embed> for EmbedDto {
    fn from(e: Embed) -> Self {
        Self {
            id: e.id,
            guild_id: e.guild_id,
            name: e.name,
            content: e.content,
            author_name: e.author_name,
            author_icon_url: e.author_icon_url,
            author_url: e.author_url,
            title: e.title,
            title_url: e.title_url,
            description: e.description,
            color: e.color,
            image_url: e.image_url,
            thumbnail_url: e.thumbnail_url,
            footer_text: e.footer_text,
            footer_icon_url: e.footer_icon_url,
            show_timestamp: e.show_timestamp,
            fields: e.fields,
            last_channel_id: e.last_channel_id,
            last_message_id: e.last_message_id,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}
