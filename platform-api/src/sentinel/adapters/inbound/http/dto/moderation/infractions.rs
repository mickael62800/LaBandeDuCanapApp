use platform_core::sentinel::domain::entities::moderation::infraction::Infraction;
use platform_core::sentinel::domain::entities::system::discord_ids::ChannelId;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::MessageId;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct InfractionQueryParams {
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct InfractionResponseDto {
    pub id: String,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub username: String,
    /// Pseudo serveur (nickname) si l'user en a un. Null sinon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub message_id: MessageId,
    pub content: String,
    pub score: f64,
    pub action: String,
    pub reason: String,
    pub duration: Option<u64>,
    pub created_at: String,
}

impl From<Infraction> for InfractionResponseDto {
    fn from(inf: Infraction) -> Self {
        Self {
            id: inf.id.to_string(),
            guild_id: inf.guild_id,
            channel_id: inf.channel_id,
            user_id: inf.user_id,
            username: inf.username,
            display_name: inf.display_name,
            message_id: inf.message_id,
            content: inf.content,
            score: inf.score,
            action: inf.action.as_str().to_string(),
            reason: inf.reason,
            duration: inf.duration,
            created_at: inf.created_at.to_rfc3339(),
        }
    }
}

#[cfg(test)]
#[path = "tests/infractions.rs"]
mod tests;
