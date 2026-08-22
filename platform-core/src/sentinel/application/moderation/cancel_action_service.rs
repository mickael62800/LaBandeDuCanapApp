//! Orchestration de l'annulation d'une action de moderation.
//!
//! Extrait du handler HTTP `DELETE /api/moderation/actions/{id}` au moment ou
//! le bot est passe en gRPC : les deux adaptateurs appellent ce service, il n'y
//! a donc qu'une seule definition de « ce que veut dire annuler une sanction ».

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, warn};

use crate::sentinel::domain::entities::moderation::action::reversal::{
    reversal_effect, ReversalEffect,
};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::cancel_action::{
    CancelModerationActionUseCase, CancelOutcome,
};
use crate::sentinel::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use crate::sentinel::ports::outbound::discord_api::DiscordApi;

pub struct CancelModerationActionService {
    moderation_uc: Arc<dyn ManageModerationUseCase>,
    discord_api: Arc<dyn DiscordApi>,
}

impl CancelModerationActionService {
    pub fn new(
        moderation_uc: Arc<dyn ManageModerationUseCase>,
        discord_api: Arc<dyn DiscordApi>,
    ) -> Self {
        Self {
            moderation_uc,
            discord_api,
        }
    }
}

#[async_trait]
impl CancelModerationActionUseCase for CancelModerationActionService {
    async fn cancel(&self, action_id: uuid::Uuid) -> Result<CancelOutcome, DomainError> {
        let Some(info) = self
            .moderation_uc
            .find_action_for_reversal(action_id)
            .await?
        else {
            return Ok(CancelOutcome::NotFound);
        };

        // La REGLE (quel effet inverse pour quel type) vit dans le domaine ;
        // ce service n'orchestre que les appels sortants.
        match reversal_effect(&info.action_type) {
            ReversalEffect::Unban { .. } => {
                match self
                    .discord_api
                    .unban_user(&info.guild_id, &info.target_id)
                    .await
                {
                    Ok(()) => info!(
                        guild_id = %info.guild_id,
                        target_id = %info.target_id,
                        target_name = %info.target_name,
                        "Unban Discord applique lors de l'annulation d'une action ban"
                    ),
                    Err(e) => warn!(
                        error = %e,
                        guild_id = %info.guild_id,
                        target_id = %info.target_id,
                        "Echec unban Discord lors de l'annulation — suppression en base quand meme"
                    ),
                }

                // Le ban annule peut porter un rappel d'auto-unban encore
                // `pending`. Sans cette annulation, le worker rejouerait un
                // unban tardif sur un membre potentiellement re-banni depuis.
            }
            ReversalEffect::RemoveTimeout => {
                match self
                    .discord_api
                    .remove_timeout(&info.guild_id, &info.target_id)
                    .await
                {
                    Ok(()) => info!(
                        guild_id = %info.guild_id,
                        target_id = %info.target_id,
                        target_name = %info.target_name,
                        "Timeout Discord retire lors de l'annulation"
                    ),
                    Err(e) => warn!(
                        error = %e,
                        guild_id = %info.guild_id,
                        target_id = %info.target_id,
                        "Echec remove_timeout Discord — suppression en base quand meme"
                    ),
                }
            }
            ReversalEffect::None => {}
        }

        if self.moderation_uc.delete_action(action_id).await? {
            Ok(CancelOutcome::Cancelled)
        } else {
            Ok(CancelOutcome::NotFound)
        }
    }
}

