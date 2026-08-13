//! Handlers reminders — adaptateur HTTP mince sur `ModerationState.manage_reminders_uc`.
//!
//! Surface HTTP supprimee lors d'un nettoyage trop large ; le use-case
//! (`ManageRemindersUseCase`) et les DTO avaient survecu.

use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::State;
use axum::Json;

use crate::sentinel::adapters::inbound::http::dto::moderation::reminders::CreateReminderDto;
use crate::sentinel::adapters::inbound::http::dto::moderation::reminders::SanctionReminderDto;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::helpers::map_to_dtos;
use crate::sentinel::adapters::inbound::http::helpers::single_dto;
use crate::sentinel::bootstrap::state::ModerationState;

/// POST /api/reminders
pub async fn create_reminder(
    State(state): State<ModerationState>,
    Json(dto): Json<CreateReminderDto>,
) -> Result<Json<SanctionReminderDto>, ApiError> {
    let command = dto.into();
    let reminder = state.manage_reminders_uc.create_reminder(command).await?;
    Ok(single_dto(reminder))
}

/// GET /api/reminders/pending
pub async fn get_pending(
    State(state): State<ModerationState>,
) -> Result<Json<Vec<SanctionReminderDto>>, ApiError> {
    let reminders = state.manage_reminders_uc.get_pending_reminders().await?;
    Ok(map_to_dtos(reminders))
}

/// GET /api/reminders/{guild_id}
pub async fn list_by_guild(
    State(state): State<ModerationState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<SanctionReminderDto>>, ApiError> {
    let reminders = state.manage_reminders_uc.list_by_guild(&guild_id).await?;
    Ok(map_to_dtos(reminders))
}
