//! Client API du voice module.
//!
//! Phase 7A -- Migration gRPC :
//! - VoiceChannels CRUD (list, create, delete, update, get, transfer,
//!   add_co_admin, add_to_whitelist, ban_user) -> `VoiceChannelsService`
//! - `log_moderation_action` -> reuse `ModerationService.LogAction`

use std::sync::Arc;

use crate::shared::grpc_client::SentinelGrpcClient;
use serde::{Deserialize, Serialize};

use platform_proto::sentinel::moderation::v1 as proto_mod;
use platform_proto::sentinel::voice::v1 as proto;

// ── Request DTOs (surface inchangee) ──

#[derive(Debug, Serialize)]
pub struct CreateVoiceChannelRequest {
    pub guild_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub channel_id: String,
    pub text_channel_id: Option<String>,
    pub members_channel_id: Option<String>,
    pub queue_channel_id: Option<String>,
    pub category_id: Option<String>,
    pub channel_name: String,
    pub kind: String,
    pub visibility: String,
    pub queue_enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct UpdateVoiceChannelRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_limit: Option<Option<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_channel_id: Option<Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct TransferOwnershipRequest {
    pub new_owner_id: String,
    pub new_owner_name: String,
}

#[derive(Debug, Serialize)]
pub struct AddCoAdminRequest {
    pub user_id: String,
    pub user_name: String,
}

#[derive(Debug, Serialize)]
pub struct AddWhitelistRequest {
    pub guild_id: String,
    pub owner_id: String,
    pub target_id: String,
    pub target_name: String,
}

#[derive(Debug, Serialize)]
pub struct SavePresetRequest {
    pub owner_id: String,
    pub channel_name: Option<String>,
    pub member_limit: Option<i32>,
    pub visibility: String,
    pub locked: bool,
    pub queue_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct VoicePresetResponse {
    pub channel_name: Option<String>,
    pub member_limit: Option<i32>,
    pub visibility: String,
    pub locked: bool,
}

#[derive(Debug, Deserialize)]
pub struct WhitelistEntryResponse {
    pub target_id: String,
}

#[derive(Debug, Deserialize)]
pub struct OwnerBanResponse {
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct BanFromChannelRequest {
    pub user_id: String,
    pub user_name: String,
    pub banned_by: String,
    pub reason: Option<String>,
    pub duration_secs: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct LogModerationActionRequest {
    pub guild_id: String,
    pub channel_id: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub duration: Option<i64>,
}

// ── Response DTOs (surface inchangee) ──

#[derive(Debug, Deserialize)]
pub struct VoiceChannelResponse {
    pub id: String,
    pub owner_id: String,
    pub channel_id: String,
    pub text_channel_id: Option<String>,
    pub members_channel_id: Option<String>,
    pub queue_channel_id: Option<String>,
    pub category_id: Option<String>,
    pub channel_name: String,
    pub visibility: String,
    pub queue_enabled: bool,
    pub locked: bool,
    pub member_limit: Option<i32>,
}

// ── Client ──

pub struct ApiClient {
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    /// Construction classique : prend le `BaseApiClient` HTTP (legacy, garde
    /// pour compat/heartbeat) et le `SentinelGrpcClient` (Phase 7A).
    pub fn new(grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { grpc }
    }

    /// Helper : construit un `ApiClient` depuis le TypeMap Serenity. Renvoie
    /// `None` si l'un des deux clients n'a pas ete insere dans `main.rs`.
    pub fn from_data(data: &serenity::prelude::TypeMap) -> Option<Self> {
        let grpc = data
            .get::<crate::shared::grpc_client::GrpcClientKey>()?
            .clone();
        Some(Self::new(grpc))
    }

    // ── Channels (gRPC) ──

    pub async fn list_channels(&self, guild_id: &str) -> Result<Vec<VoiceChannelResponse>, String> {
        let req = proto::ListChannelsRequest {
            guild_id: guild_id.to_string(),
        };
        let list = crate::grpc_call!(self.grpc, voice_channels, list_channels, req)?;
        Ok(list.channels.into_iter().map(proto_to_response).collect())
    }

    pub async fn create_channel(
        &self,
        request: &CreateVoiceChannelRequest,
    ) -> Result<VoiceChannelResponse, String> {
        let req = proto::CreateChannelRequest {
            guild_id: request.guild_id.clone(),
            owner_id: request.owner_id.clone(),
            owner_name: request.owner_name.clone(),
            channel_id: request.channel_id.clone(),
            text_channel_id: request.text_channel_id.clone(),
            members_channel_id: request.members_channel_id.clone(),
            queue_channel_id: request.queue_channel_id.clone(),
            category_id: request.category_id.clone(),
            channel_name: request.channel_name.clone(),
            kind: request.kind.clone(),
            visibility: request.visibility.clone(),
            queue_enabled: request.queue_enabled,
        };
        let c = crate::grpc_call!(self.grpc, voice_channels, create_channel, req)?;
        Ok(proto_to_response(c))
    }

    pub async fn delete_channel(&self, channel_id: &str) -> Result<(), String> {
        let req = proto::DeleteChannelRequest {
            channel_id: channel_id.to_string(),
        };
        crate::grpc_call!(@unit self.grpc, voice_channels, delete_channel, req)
    }

    pub async fn update_channel(
        &self,
        channel_id: &str,
        request: &UpdateVoiceChannelRequest,
    ) -> Result<(), String> {
        let req = proto::UpdateChannelRequest {
            channel_id: channel_id.to_string(),
            visibility: request.visibility.clone(),
            locked: request.locked,
            queue_enabled: request.queue_enabled,
            name: request.name.clone(),
            status: request.status.clone(),
            member_limit: request
                .member_limit
                .map(|opt| proto::MemberLimitUpdate { value: opt }),
            queue_channel_id: request
                .queue_channel_id
                .clone()
                .map(|opt| proto::QueueChannelUpdate { value: opt }),
        };
        crate::grpc_call!(@unit self.grpc, voice_channels, update_channel, req)
    }

    pub async fn get_channel(
        &self,
        channel_id: &str,
    ) -> Result<Option<VoiceChannelResponse>, String> {
        let req = proto::GetChannelRequest {
            channel_id: channel_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, voice_channels, get_channel, req)?;
        Ok(resp.channel.map(proto_to_response))
    }

    /// Retourne le channel + la liste des co-admins (user_ids).
    pub async fn get_channel_co_admins(&self, channel_id: &str) -> Result<Vec<String>, String> {
        let req = proto::GetChannelRequest {
            channel_id: channel_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, voice_channels, get_channel, req)?;
        Ok(resp.co_admins.into_iter().map(|ca| ca.user_id).collect())
    }

    // ── Transfer ──

    pub async fn transfer_ownership(
        &self,
        channel_id: &str,
        request: &TransferOwnershipRequest,
    ) -> Result<(), String> {
        let req = proto::TransferOwnershipRequest {
            channel_id: channel_id.to_string(),
            new_owner_id: request.new_owner_id.clone(),
            new_owner_name: request.new_owner_name.clone(),
        };
        crate::grpc_call!(@unit self.grpc, voice_channels, transfer_ownership, req)
    }

    // ── Co-admins ──

    pub async fn add_co_admin(
        &self,
        channel_id: &str,
        request: &AddCoAdminRequest,
    ) -> Result<(), String> {
        let req = proto::AddCoAdminRequest {
            channel_id: channel_id.to_string(),
            user_id: request.user_id.clone(),
            user_name: request.user_name.clone(),
        };
        crate::grpc_call!(@unit self.grpc, voice_channels, add_co_admin, req)
    }

    // ── Whitelist ──

    pub async fn add_to_whitelist(&self, request: &AddWhitelistRequest) -> Result<(), String> {
        let req = proto::AddToWhitelistRequest {
            guild_id: request.guild_id.clone(),
            owner_id: request.owner_id.clone(),
            target_id: request.target_id.clone(),
            target_name: request.target_name.clone(),
        };
        crate::grpc_call!(@unit self.grpc, voice_channels, add_to_whitelist, req)
    }

    // ── Presets + whitelist (HTTP via BaseApiClient) ──

    /// Lit le preset memorise par ce proprietaire (gRPC). `None` si aucun.
    pub async fn get_preset(&self, guild_id: &str, owner_id: &str) -> Option<VoicePresetResponse> {
        let req = proto::GetPresetRequest {
            guild_id: guild_id.to_string(),
            owner_id: owner_id.to_string(),
        };
        let resp = crate::grpc_call!(@raw self.grpc, voice_channels, get_preset, req).ok()?;
        resp.preset.map(|p| VoicePresetResponse {
            channel_name: p.channel_name,
            member_limit: p.member_limit,
            visibility: p.visibility,
            locked: p.locked,
        })
    }

    /// Cree ou met a jour le preset du proprietaire (gRPC). Tolerant aux erreurs.
    pub async fn save_preset(&self, guild_id: &str, request: &SavePresetRequest) {
        let req = proto::SavePresetRequest {
            guild_id: guild_id.to_string(),
            owner_id: request.owner_id.clone(),
            channel_name: request.channel_name.clone(),
            member_limit: request.member_limit,
            visibility: request.visibility.clone(),
            locked: request.locked,
            queue_enabled: request.queue_enabled,
        };
        if let Err(e) = crate::grpc_call!(@raw_unit self.grpc, voice_channels, save_preset, req) {
            tracing::warn!(error = %grpc_err_to_string(e), "Echec save_preset gRPC");
        }
    }

    /// Liste les membres whitelistes (amis) memorises pour ce proprietaire (gRPC).
    pub async fn get_whitelist(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Vec<WhitelistEntryResponse> {
        let req = proto::GetWhitelistRequest {
            guild_id: guild_id.to_string(),
            owner_id: owner_id.to_string(),
        };
        match crate::grpc_call!(@raw self.grpc, voice_channels, get_whitelist, req) {
            Ok(list) => list
                .entries
                .into_iter()
                .map(|e| WhitelistEntryResponse {
                    target_id: e.target_id,
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    // ── Bans ──

    pub async fn ban_user(
        &self,
        channel_id: &str,
        request: &BanFromChannelRequest,
    ) -> Result<(), String> {
        let req = proto::BanFromChannelRequest {
            channel_id: channel_id.to_string(),
            user_id: request.user_id.clone(),
            user_name: request.user_name.clone(),
            banned_by: request.banned_by.clone(),
            reason: request.reason.clone(),
            duration_secs: request.duration_secs,
        };
        crate::grpc_call!(@unit self.grpc, voice_channels, ban_from_channel, req)
    }

    /// Verifie si un user est banni pour le proprietaire du salon (join-time).
    /// Tolerant aux erreurs : en cas d'echec gRPC on renvoie `false` (fail-open)
    /// pour ne jamais deconnecter a tort sur une indisponibilite API.
    pub async fn is_banned(&self, channel_id: &str, user_id: &str) -> bool {
        let req = proto::IsBannedRequest {
            channel_id: channel_id.to_string(),
            user_id: user_id.to_string(),
        };
        match crate::grpc_call!(@raw self.grpc, voice_channels, is_banned, req) {
            Ok(resp) => resp.banned,
            Err(_) => false,
        }
    }

    /// Liste les bans memorises pour ce proprietaire (re-application a la
    /// recreation du salon). Tolerant aux erreurs : liste vide si echec.
    pub async fn list_owner_bans(&self, guild_id: &str, owner_id: &str) -> Vec<OwnerBanResponse> {
        let req = proto::ListOwnerBansRequest {
            guild_id: guild_id.to_string(),
            owner_id: owner_id.to_string(),
        };
        match crate::grpc_call!(@raw self.grpc, voice_channels, list_owner_bans, req) {
            Ok(list) => list
                .bans
                .into_iter()
                .map(|b| OwnerBanResponse { user_id: b.user_id })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    // ── Moderation log (reuse ModerationService.LogAction) ──

    pub async fn log_moderation_action(
        &self,
        request: &LogModerationActionRequest,
    ) -> Result<(), String> {
        let req = proto_mod::LogActionRequest {
            guild_id: request.guild_id.clone(),
            channel_id: request.channel_id.clone(),
            moderator_id: request.moderator_id.clone(),
            moderator_name: request.moderator_name.clone(),
            target_id: request.target_id.clone(),
            target_name: request.target_name.clone(),
            action_type: request.action_type.clone(),
            reason: request.reason.clone(),
            gravity: None,
            duration: request.duration.map(|d| d as u64),
            skip_strike: false,
        };
        crate::grpc_call!(@unit self.grpc, moderation, log_action, req)
    }

    // ── Config voice-bot par guild (gRPC) ──

    pub async fn get_voice_config(&self, guild_id: &str) -> Result<VoiceConfigResponse, String> {
        let req = proto::GetVoiceConfigRequest {
            guild_id: guild_id.to_string(),
        };
        let cfg = crate::grpc_call!(self.grpc, voice_channels, get_voice_config, req)?;
        Ok(VoiceConfigResponse {
            creation_cooldown_secs: cfg.creation_cooldown_secs,
            flood_max_messages: cfg.flood_max_messages,
            flood_time_window_secs: cfg.flood_time_window_secs,
            empty_cleanup_delay_secs: cfg.empty_cleanup_delay_secs,
            flood_mute_duration_secs: cfg.flood_mute_duration_secs,
        })
    }

    // ── Themes (gRPC) ──

    pub async fn list_themes(&self, guild_id: &str) -> Result<Vec<VoiceThemeResponse>, String> {
        let req = proto::ListThemesRequest {
            guild_id: guild_id.to_string(),
        };
        let list = crate::grpc_call!(self.grpc, voice_channels, list_themes, req)?;
        Ok(list
            .themes
            .into_iter()
            .map(|t| VoiceThemeResponse {
                name: t.name,
                member_limit: t.member_limit,
            })
            .collect())
    }
}

// ── Response DTOs ──

#[derive(Debug, Clone)]
pub struct VoiceConfigResponse {
    pub creation_cooldown_secs: u64,
    pub flood_max_messages: u64,
    pub flood_time_window_secs: u64,
    pub empty_cleanup_delay_secs: u64,
    pub flood_mute_duration_secs: u64,
}

#[derive(Debug, Clone)]
pub struct VoiceThemeResponse {
    pub name: String,
    pub member_limit: Option<i32>,
}

fn proto_to_response(c: proto::VoiceChannel) -> VoiceChannelResponse {
    VoiceChannelResponse {
        id: c.id,
        owner_id: c.owner_id,
        channel_id: c.channel_id,
        text_channel_id: c.text_channel_id,
        members_channel_id: c.members_channel_id,
        queue_channel_id: c.queue_channel_id,
        category_id: c.category_id,
        channel_name: c.channel_name,
        visibility: c.visibility,
        queue_enabled: c.queue_enabled,
        locked: c.locked,
        member_limit: c.member_limit,
    }
}

use crate::shared::grpc_client::grpc_err_to_string;
