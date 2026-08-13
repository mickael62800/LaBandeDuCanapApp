//! Client API du welcome-bot.
//!
//! Phase 7A — Migration gRPC :
//! - `is_known_member` -> `MembersService.GetMember` (hot path : a chaque
//!   nouveau membre rejoignant un serveur).
//! - `get_config` -> `WelcomeService.GetConfig` (Phase 7A.opt F.4).

use std::sync::Arc;

use crate::shared::grpc_client::{GrpcCallError, SentinelGrpcClient};

use platform_proto::sentinel::members::v1 as proto_members;
use platform_proto::sentinel::welcome::v1 as proto_welcome;

#[derive(Debug)]
/// Miroir de la config welcome renvoyee par l'API.
///
/// TODO(mort) : les 6 champs `anniversary_*` ne sont lus par AUCUN handler du
/// bot — la fonctionnalite `anniversaire d'arrivee` est configurable cote web
/// mais n'est jamais rendue sur Discord. Meme classe de probleme que la
/// sauvegarde automatique de `guild_backup`. Champs conserves : ils
/// documentent le contrat de l'API.
#[allow(dead_code)]
pub struct WelcomeConfig {
    pub guild_id: String,
    pub welcome_enabled: bool,
    pub welcome_channel_id: Option<String>,
    pub welcome_message: String,
    pub welcome_embed_color: String,
    pub welcome_dm_enabled: bool,
    pub welcome_dm_message: String,
    pub leave_enabled: bool,
    pub leave_channel_id: Option<String>,
    pub leave_message: String,
    pub rules_enabled: bool,
    pub rules_channel_id: Option<String>,
    pub rules_message: String,
    pub rules_role_id: Option<String>,
    pub rules_button_label: String,
    pub age_check_enabled: bool,
    pub age_minimum: i32,
    pub unverified_role_id: Option<String>,
    pub age_modal_question: String,
    pub age_ban_message: String,
    pub counter_enabled: bool,
    pub counter_channel_id: Option<String>,
    pub counter_format: String,
    pub voice_counter_enabled: bool,
    pub voice_counter_channel_id: Option<String>,
    pub voice_counter_format: String,
    pub anniversary_enabled: bool,
    pub anniversary_channel_id: Option<String>,
    pub anniversary_message: String,
    pub rejoin_message: String,
    pub welcome_title: String,
    pub welcome_image_url: String,
    pub welcome_footer_text: String,
    pub rejoin_title: String,
    pub rejoin_image_url: String,
    pub rejoin_footer_text: String,
    pub leave_title: String,
    pub leave_image_url: String,
    pub leave_footer_text: String,
    pub anniversary_title: String,
    pub anniversary_image_url: String,
    pub anniversary_footer_text: String,
}

impl From<proto_welcome::WelcomeConfig> for WelcomeConfig {
    fn from(p: proto_welcome::WelcomeConfig) -> Self {
        Self {
            guild_id: p.guild_id,
            welcome_enabled: p.welcome_enabled,
            welcome_channel_id: p.welcome_channel_id,
            welcome_message: p.welcome_message,
            welcome_embed_color: p.welcome_embed_color,
            welcome_dm_enabled: p.welcome_dm_enabled,
            welcome_dm_message: p.welcome_dm_message,
            leave_enabled: p.leave_enabled,
            leave_channel_id: p.leave_channel_id,
            leave_message: p.leave_message,
            rules_enabled: p.rules_enabled,
            rules_channel_id: p.rules_channel_id,
            rules_message: p.rules_message,
            rules_role_id: p.rules_role_id,
            rules_button_label: p.rules_button_label,
            age_check_enabled: p.age_check_enabled,
            age_minimum: p.age_minimum,
            unverified_role_id: p.unverified_role_id,
            age_modal_question: p.age_modal_question,
            age_ban_message: p.age_ban_message,
            counter_enabled: p.counter_enabled,
            counter_channel_id: p.counter_channel_id,
            counter_format: p.counter_format,
            voice_counter_enabled: p.voice_counter_enabled,
            voice_counter_channel_id: p.voice_counter_channel_id,
            voice_counter_format: p.voice_counter_format,
            anniversary_enabled: p.anniversary_enabled,
            anniversary_channel_id: p.anniversary_channel_id,
            anniversary_message: p.anniversary_message,
            rejoin_message: p.rejoin_message,
            welcome_title: p.welcome_title,
            welcome_image_url: p.welcome_image_url,
            welcome_footer_text: p.welcome_footer_text,
            rejoin_title: p.rejoin_title,
            rejoin_image_url: p.rejoin_image_url,
            rejoin_footer_text: p.rejoin_footer_text,
            leave_title: p.leave_title,
            leave_image_url: p.leave_image_url,
            leave_footer_text: p.leave_footer_text,
            anniversary_title: p.anniversary_title,
            anniversary_image_url: p.anniversary_image_url,
            anniversary_footer_text: p.anniversary_footer_text,
        }
    }
}

pub struct WelcomeApiClient {
    // Conserve pour compat TypeMap (heartbeat reste HTTP).
    grpc: Arc<SentinelGrpcClient>,
}

impl WelcomeApiClient {
    pub fn new(grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { grpc }
    }

    /// gRPC `WelcomeService.GetConfig` (Phase 7A.opt F.4).
    pub async fn get_config(&self, guild_id: &str) -> Result<WelcomeConfig, String> {
        let req = proto_welcome::GetConfigRequest {
            guild_id: guild_id.to_string(),
        };
        let mut client = self.grpc.welcome();
        let cfg = self
            .grpc
            .guarded(|| async move { client.get_config(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(|e| match e {
                GrpcCallError::Unavailable => {
                    "API indisponible (circuit breaker ouvert)".to_string()
                }
                GrpcCallError::Status(s) => format!("gRPC {:?}: {}", s.code(), s.message()),
                GrpcCallError::Transport(t) => format!("transport gRPC: {t}"),
            })?;
        Ok(cfg.into())
    }

    /// gRPC `MembersService.GetMember` (hot path).
    /// Renvoie `false` si le membre n'existe pas (parite avec l'ancien 404 HTTP).
    pub async fn is_known_member(&self, guild_id: &str, user_id: &str) -> bool {
        let req = proto_members::GetMemberRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let result = crate::grpc_call!(@raw self.grpc, members, get_member, req);
        match result {
            Ok(resp) => resp.member.is_some(),
            Err(_) => false,
        }
    }
}
