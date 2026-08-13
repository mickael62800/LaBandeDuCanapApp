//! Phase 5G — Endpoints `security_lockdown_active` (adaptateur ENTRANT mince).
//! La regle metier (calcul de l'expiration) vit dans `ManageLockdownUseCase`, le
//! SQL dans `LockdownRepository`. Ici : parse -> use case -> map.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use crate::sentinel::bootstrap::state::SystemState;

#[derive(Deserialize)]
pub struct CreateLockdownDto {
    pub guild_id: String,
    /// JSON array decrivant les overwrites originaux par salon.
    /// Cf domains/security/expire_lockdown.rs cote worker pour le format.
    pub saved_states: serde_json::Value,
    pub duration_secs: i64,
}

/// POST /api/security/lockdown — bot enregistre un lockdown actif.
/// UPSERT pour idempotence (re-activation reset le timer + states).
pub async fn create_lockdown(
    State(state): State<SystemState>,
    Json(dto): Json<CreateLockdownDto>,
) -> Result<StatusCode, ApiError> {
    state
        .lockdown_uc
        .activate(&dto.guild_id, dto.saved_states, dto.duration_secs)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/security/lockdown/{guild_id} — bot retire un lockdown
/// (deactivation manuelle ou via worker).
pub async fn delete_lockdown(
    State(state): State<SystemState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<StatusCode, ApiError> {
    state.lockdown_uc.deactivate(&guild_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
