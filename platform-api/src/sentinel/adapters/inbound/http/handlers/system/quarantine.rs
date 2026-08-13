//! Phase 5F — Endpoints `security_quarantine_pending` (adaptateur ENTRANT mince).
//! La regle metier (delai avant kick) vit dans `ManageQuarantineUseCase`, le SQL
//! dans `QuarantineRepository`. Ici : parse -> use case -> map.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuildUser;
use crate::sentinel::bootstrap::state::SystemState;

#[derive(Deserialize)]
pub struct CreateQuarantineDto {
    pub guild_id: String,
    pub user_id: String,
    /// Duree avant kick automatique (secondes).
    pub timeout_secs: i64,
}

/// POST /api/security/quarantine — bot enregistre la mise en quarantaine
/// d'un user. UPSERT pour idempotence (re-quarantaine reset le timer).
pub async fn create_quarantine(
    State(state): State<SystemState>,
    Json(dto): Json<CreateQuarantineDto>,
) -> Result<StatusCode, ApiError> {
    state
        .quarantine_uc
        .quarantine_user(&dto.guild_id, &dto.user_id, dto.timeout_secs)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/security/quarantine/active — liste les quarantaines encore actives
/// (non expirees). Le bot l'appelle au demarrage pour rehydrater son tracker RAM
/// (sinon, apres un reboot, un user quarantine ne peut plus se verifier et sa
/// quarantaine ne peut plus etre levee cote bot).
pub async fn list_active_quarantines(
    State(state): State<SystemState>,
) -> Result<Json<Vec<(String, String)>>, ApiError> {
    let rows = state.quarantine_uc.list_active().await?;
    Ok(Json(
        rows.into_iter().map(|q| (q.guild_id, q.user_id)).collect(),
    ))
}

/// DELETE /api/security/quarantine/{guild_id}/{user_id} — bot retire un
/// user de la quarantaine apres validation captcha (ou suppression par
/// admin). Idempotent.
pub async fn delete_quarantine(
    State(state): State<SystemState>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<StatusCode, ApiError> {
    state.quarantine_uc.lift(&guild_id, &user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
