use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use chrono::NaiveDate;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DailyActivity {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub day: NaiveDate,
    pub messages: i64,
    pub voice_minutes: i64,
    pub active_members: i32,
    pub new_members: i32,
    pub leaves: i32,
    pub infractions: i32,
    pub warns: i32,
    pub mutes: i32,
    pub bans: i32,
}
