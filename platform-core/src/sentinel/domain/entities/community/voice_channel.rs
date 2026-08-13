use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::sentinel::domain::entities::system::discord_ids::ChannelId;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use crate::sentinel::domain::enums::community::voice_channel_kind::VoiceChannelKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannel {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub owner_id: String,
    pub owner_name: String,
    pub channel_id: ChannelId,
    pub text_channel_id: Option<String>,
    pub members_channel_id: Option<String>,
    pub queue_channel_id: Option<String>,
    pub category_id: Option<String>,
    pub channel_name: String,
    pub kind: VoiceChannelKind,
    pub visibility: String,
    pub queue_enabled: bool,
    pub locked: bool,
    pub stage_enabled: bool,
    pub member_limit: Option<i32>,
    pub status: Option<String>,
    pub channel_status: String,
    pub closed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelCoAdmin {
    pub id: Uuid,
    pub voice_channel_id: Uuid,
    pub user_id: UserId,
    pub user_name: String,
    pub granted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelWhitelistEntry {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub owner_id: String,
    pub target_id: String,
    pub target_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelBan {
    pub id: Uuid,
    /// Reference historique vers l'instance du salon au moment du ban. Peut
    /// pointer vers un salon supprime : la cle logique est desormais
    /// (guild_id, owner_id, user_id).
    pub voice_channel_id: Uuid,
    pub guild_id: GuildId,
    pub owner_id: String,
    pub user_id: UserId,
    pub user_name: String,
    pub banned_by: String,
    pub reason: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelInviteLink {
    pub id: Uuid,
    pub voice_channel_id: Uuid,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub created_by: String,
    pub created_by_name: String,
    pub code: String,
    pub max_uses: Option<i32>,
    pub current_uses: i32,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelTheme {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub name: String,
    pub emoji: Option<String>,
    pub channel_name_template: String,
    pub member_limit: Option<i32>,
    pub visibility: String,
    pub locked: bool,
    pub queue_enabled: bool,
    pub bitrate: Option<i32>,
    pub slowmode_secs: Option<i32>,
    pub stage_enabled: bool,
    pub is_default: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

/// Preset de parametres memorise par proprietaire. Reapplique a la creation
/// d'un nouveau salon temporaire (le bouton "Sauvegarder mes parametres" du
/// panneau de controle persiste l'etat courant du salon ici).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelPreset {
    pub guild_id: GuildId,
    pub owner_id: String,
    pub channel_name: Option<String>,
    pub member_limit: Option<i32>,
    pub visibility: String,
    pub locked: bool,
    pub queue_enabled: bool,
    pub updated_at: DateTime<Utc>,
}

/// Configuration voice-bot par guild. Valeurs lues depuis `bot_guild_config`
/// (bot_name = "voice-bot"), avec fallback sur les defaults raisonnables.
#[derive(Debug, Clone, Copy)]
pub struct VoiceChannelConfig {
    /// Cooldown entre deux creations de salon par un meme user (V2).
    pub creation_cooldown_secs: u64,
    /// Seuil de flood : nombre de messages dans la fenetre (V3).
    pub flood_max_messages: u64,
    /// Fenetre de detection de flood en secondes (V3).
    pub flood_time_window_secs: u64,
    /// Delai anti-race avant suppression d'un salon vide (V4).
    pub empty_cleanup_delay_secs: u64,
    /// Duree du mute automatique sur flood detecte (V8).
    pub flood_mute_duration_secs: u64,
    /// Duree du vote-kick avant expiration (V10).
    pub vote_kick_timeout_secs: u64,
}

impl Default for VoiceChannelConfig {
    fn default() -> Self {
        Self {
            creation_cooldown_secs: 5,
            flood_max_messages: 5,
            flood_time_window_secs: 5,
            empty_cleanup_delay_secs: 2,
            flood_mute_duration_secs: 30,
            vote_kick_timeout_secs: 60,
        }
    }
}

impl VoiceChannelConfig {
    /// Construit depuis une liste de `(key, value)` lues en DB.
    pub fn from_kv_pairs(pairs: &[(String, String)]) -> Self {
        let mut cfg = Self::default();
        for (k, v) in pairs {
            match k.as_str() {
                "voice_creation_cooldown_secs" => {
                    if let Ok(n) = v.parse() {
                        cfg.creation_cooldown_secs = n;
                    }
                }
                "voice_flood_max_messages" => {
                    if let Ok(n) = v.parse() {
                        cfg.flood_max_messages = n;
                    }
                }
                "voice_flood_time_window_secs" => {
                    if let Ok(n) = v.parse() {
                        cfg.flood_time_window_secs = n;
                    }
                }
                "voice_empty_cleanup_delay_secs" => {
                    if let Ok(n) = v.parse() {
                        cfg.empty_cleanup_delay_secs = n;
                    }
                }
                "voice_flood_mute_duration_secs" => {
                    if let Ok(n) = v.parse() {
                        cfg.flood_mute_duration_secs = n;
                    }
                }
                "voice_vote_kick_timeout_secs" => {
                    if let Ok(n) = v.parse() {
                        cfg.vote_kick_timeout_secs = n;
                    }
                }
                _ => {}
            }
        }
        cfg
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelDetail {
    pub channel: VoiceChannel,
    pub co_admins: Vec<VoiceChannelCoAdmin>,
    pub bans: Vec<VoiceChannelBan>,
    pub invite_links: Vec<VoiceChannelInviteLink>,
}

#[cfg(test)]
#[path = "tests/voice_channel.rs"]
mod tests;
