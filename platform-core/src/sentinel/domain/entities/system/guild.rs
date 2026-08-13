use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guild {
    pub guild_id: GuildId,
    pub name: String,
    pub icon: Option<String>,
    pub member_count: i32,
    pub registered_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
