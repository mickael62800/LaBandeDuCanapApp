use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::domain::entities::system::discord_ids::ChannelId;
use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::enums::moderation::moderation_gravity::ModerationGravity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationAction {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    /// Pseudo serveur (nickname) actuel de la cible si elle est encore dans
    /// la guild. Lu via LEFT JOIN guild_members.display_name. Optionnel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_display_name: Option<String>,
    pub action_type: String,
    pub reason: String,
    pub gravity: Option<ModerationGravity>,
    pub duration: Option<u64>,
    pub created_at: DateTime<Utc>,
}

impl ModerationAction {
    pub const AUDIT_EVENT_PREFIX: &'static str = "mod_";

    /// Type d'evenement canonique persiste dans `audit_logs`.
    pub fn audit_event_type(&self) -> String {
        format!("{}{}", Self::AUDIT_EVENT_PREFIX, self.action_type)
    }

    /// Payload canonique des champs propres a une action de moderation.
    pub fn audit_details(&self) -> serde_json::Value {
        serde_json::json!({
            "reason": self.reason,
            "gravity": self.gravity.as_ref().map(|gravity| gravity.as_str()),
            "duration_secs": self.duration,
            "action_id": self.id.to_string(),
        })
    }

    pub fn action_type_from_audit_event(event_type: &str) -> Option<&str> {
        event_type.strip_prefix(Self::AUDIT_EVENT_PREFIX)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserModerationHistory {
    pub target_id: String,
    pub target_name: String,
    pub total_warns: u32,
    pub total_mutes: u32,
    pub total_bans: u32,
    pub actions: Vec<ModerationAction>,
}
