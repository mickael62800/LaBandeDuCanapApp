use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNote {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub category: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
