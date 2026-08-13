//! Mapping entite metier <-> message Discord (cf. migration 175 +
//! SYNC_DISCORD_WEB_DESIGN.md).

use crate::sentinel::domain::entities::system::discord_ids::ChannelId;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::MessageId;
use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscordActionMessage {
    pub action_id: Uuid,
    pub kind: String,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub posted_at: DateTime<Utc>,
    pub last_edited_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewDiscordActionMessage {
    pub action_id: Uuid,
    pub kind: String,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
}

/// Conventions de `kind` reconnues — non exhaustif, le champ reste libre
/// pour faciliter l'ajout de nouvelles features sans toucher au domaine.
pub mod kinds {
    pub const BAN_PROPOSAL: &str = "ban_proposal";
    pub const TICKET: &str = "ticket";
    pub const ROLES_PANEL: &str = "roles_panel";
    pub const COMBAT_CHALLENGE: &str = "combat_challenge";
    pub const REVIEW_REQUEST: &str = "review_request";
}
