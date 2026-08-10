//! Handlers notes moderateurs — adaptateur HTTP mince sur `ModerationState.notes_uc`.
//!
//! Surface HTTP supprimee lors d'un nettoyage trop large ; le use-case
//! (`ManageNotesUseCase`) et les DTO avaient survecu.

use crate::adapters::inbound::http::dto::moderation::notes::AddNoteDto;
use crate::adapters::inbound::http::dto::moderation::notes::UserNoteDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuildUser;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::validation;
use crate::bootstrap::state::ModerationState;
use axum::extract::Path;
use axum::extract::State;
use axum::Json;

/// POST /api/notes
pub async fn add_note(
    State(state): State<ModerationState>,
    Json(dto): Json<AddNoteDto>,
) -> Result<Json<UserNoteDto>, ApiError> {
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &dto.user_id).map_err(ApiError)?;
    validation::validate_content(&dto.content).map_err(ApiError)?;

    let command = dto.into();
    let note = state.notes_uc.add_note(command).await?;
    Ok(single_dto(note))
}

/// GET /api/notes/{guild_id}/{user_id}
pub async fn get_notes(
    State(state): State<ModerationState>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<Vec<UserNoteDto>>, ApiError> {
    // Moderator+ requis : les notes sont sensibles (contexte interne de modo).
    let notes = state.notes_uc.get_notes(&guild_id, &user_id).await?;
    Ok(map_to_dtos(notes))
}

/// DELETE /api/notes/{id}
pub async fn delete_note(
    State(state): State<ModerationState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation de format : 422 si l'id n'est pas un UUID, plutot qu'une
    // erreur de parsing plus bas dans le repo.
    validation::parse_uuid("id", &id).map_err(ApiError)?;

    state.notes_uc.delete_note(&id).await?;
    Ok(ok_response())
}
