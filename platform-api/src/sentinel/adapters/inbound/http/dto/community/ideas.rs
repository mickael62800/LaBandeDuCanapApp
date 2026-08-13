use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::idea::{Idea, IdeaDetail, IdeaMessage};

#[derive(Debug, Deserialize)]
pub struct CreateIdeaDto {
    pub guild_id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_category")]
    pub category: String,
    pub author_id: String,
    pub author_name: String,
    #[serde(default)]
    pub channel_id: Option<String>,
}

fn default_category() -> String {
    "autre".to_string()
}

/// Decision du staff. `decided_by` / `decided_by_name` sont fournis par le bot ;
/// cote web ils sont deduits de la session (voir le handler).
#[derive(Debug, Deserialize)]
pub struct DecideIdeaDto {
    pub status: String,
    #[serde(default)]
    pub decided_by: Option<String>,
    #[serde(default)]
    pub decided_by_name: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetIdeaChannelDto {
    #[serde(default)]
    pub channel_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddIdeaMessageDto {
    pub author_name: String,
    #[serde(default = "default_author_role")]
    pub author_role: String,
    pub content: String,
}

fn default_author_role() -> String {
    "auteur".to_string()
}

/// Filtres de listing (query string).
#[derive(Debug, Deserialize)]
pub struct ListIdeasQuery {
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub author_id: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct IdeaDto {
    pub id: Uuid,
    pub guild_id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub category: String,
    pub author_id: String,
    pub author_name: String,
    pub channel_id: Option<String>,
    pub decided_by: Option<String>,
    pub decided_by_name: Option<String>,
    pub decision_reason: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Idea> for IdeaDto {
    fn from(i: Idea) -> Self {
        Self {
            id: i.id,
            guild_id: i.guild_id,
            title: i.title,
            description: i.description,
            status: i.status,
            category: i.category,
            author_id: i.author_id,
            author_name: i.author_name,
            channel_id: i.channel_id,
            decided_by: i.decided_by,
            decided_by_name: i.decided_by_name,
            decision_reason: i.decision_reason,
            decided_at: i.decided_at,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct IdeaMessageDto {
    pub id: Uuid,
    pub idea_id: Uuid,
    pub author_name: String,
    pub author_role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl From<IdeaMessage> for IdeaMessageDto {
    fn from(m: IdeaMessage) -> Self {
        Self {
            id: m.id,
            idea_id: m.idea_id,
            author_name: m.author_name,
            author_role: m.author_role,
            content: m.content,
            created_at: m.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct IdeaDetailDto {
    pub idea: IdeaDto,
    pub messages: Vec<IdeaMessageDto>,
}

impl From<IdeaDetail> for IdeaDetailDto {
    fn from(d: IdeaDetail) -> Self {
        Self {
            idea: IdeaDto::from(d.idea),
            messages: d.messages.into_iter().map(IdeaMessageDto::from).collect(),
        }
    }
}
