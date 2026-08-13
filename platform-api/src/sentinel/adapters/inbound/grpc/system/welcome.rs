//! gRPC Welcome config — delegue au use case `ManageWelcomeConfigUseCase`.
//! Respect de l'archi hexagonale : un adapter inbound (gRPC) doit
//! toujours passer par un port inbound, jamais par un repo outbound
//! directement.

use std::sync::Arc;

use platform_proto::sentinel::welcome::v1 as proto;
use platform_proto::sentinel::welcome::v1::welcome_service_server::WelcomeService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use platform_core::sentinel::ports::inbound::community::manage_welcome_config::ManageWelcomeConfigUseCase;

pub struct WelcomeGrpc {
    pub uc: Arc<dyn ManageWelcomeConfigUseCase>,
}

#[tonic::async_trait]
impl WelcomeService for WelcomeGrpc {
    async fn get_config(
        &self,
        request: Request<proto::GetConfigRequest>,
    ) -> Result<Response<proto::WelcomeConfig>, Status> {
        let cfg = self
            .uc
            .get(&request.into_inner().guild_id)
            .await
            .map_err(|e| Status::internal(format!("get welcome config: {e}")))?;

        Ok(Response::new(proto::WelcomeConfig {
            guild_id: cfg.guild_id.into(),
            welcome_enabled: cfg.welcome_enabled,
            welcome_channel_id: cfg.welcome_channel_id,
            welcome_message: cfg.welcome_message,
            welcome_embed_color: cfg.welcome_embed_color,
            welcome_dm_enabled: cfg.welcome_dm_enabled,
            welcome_dm_message: cfg.welcome_dm_message,
            leave_enabled: cfg.leave_enabled,
            leave_channel_id: cfg.leave_channel_id,
            leave_message: cfg.leave_message,
            rules_enabled: cfg.rules_enabled,
            rules_channel_id: cfg.rules_channel_id,
            rules_message: cfg.rules_message,
            rules_role_id: cfg.rules_role_id,
            rules_button_label: cfg.rules_button_label,
            age_check_enabled: cfg.age_check_enabled,
            age_minimum: cfg.age_minimum,
            unverified_role_id: cfg.unverified_role_id,
            age_modal_question: cfg.age_modal_question,
            age_ban_message: cfg.age_ban_message,
            counter_enabled: cfg.counter_enabled,
            counter_channel_id: cfg.counter_channel_id,
            counter_format: cfg.counter_format,
            voice_counter_enabled: cfg.voice_counter_enabled,
            voice_counter_channel_id: cfg.voice_counter_channel_id,
            voice_counter_format: cfg.voice_counter_format,
            anniversary_enabled: cfg.anniversary_enabled,
            anniversary_channel_id: cfg.anniversary_channel_id,
            anniversary_message: cfg.anniversary_message,
            rejoin_message: cfg.rejoin_message,
            welcome_title: cfg.welcome_title,
            welcome_image_url: cfg.welcome_image_url,
            welcome_footer_text: cfg.welcome_footer_text,
            rejoin_title: cfg.rejoin_title,
            rejoin_image_url: cfg.rejoin_image_url,
            rejoin_footer_text: cfg.rejoin_footer_text,
            leave_title: cfg.leave_title,
            leave_image_url: cfg.leave_image_url,
            leave_footer_text: cfg.leave_footer_text,
            anniversary_title: cfg.anniversary_title,
            anniversary_image_url: cfg.anniversary_image_url,
            anniversary_footer_text: cfg.anniversary_footer_text,
        }))
    }
}

#[cfg(test)]
#[path = "tests/welcome.rs"]
mod tests;
