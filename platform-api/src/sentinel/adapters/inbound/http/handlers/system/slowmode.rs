//! Phase 5H — Endpoints `security_slowmode_active` (adaptateur ENTRANT mince).
//! La regle metier (calcul de l'expiration) vit dans `ManageSlowmodeUseCase`, le
//! SQL dans `SlowmodeRepository`. Ici : parse -> use case -> map.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use crate::sentinel::bootstrap::state::SystemState;

#[derive(Deserialize)]
pub struct CreateSlowmodeDto {
    pub guild_id: String,
    /// JSON array : [{channel_id, rate}, ...]
    pub previous_states: serde_json::Value,
    pub duration_secs: i64,
    /// Rate impose par le raid (pour ne restaurer que si le salon le porte encore).
    #[serde(default)]
    pub imposed_rate: i32,
}

/// POST /api/security/slowmode — bot enregistre un slowmode actif.
/// UPSERT pour idempotence (re-activation reset le timer + states).
pub async fn create_slowmode(
    State(state): State<SystemState>,
    Json(dto): Json<CreateSlowmodeDto>,
) -> Result<StatusCode, ApiError> {
    state
        .slowmode_uc
        .activate(
            &dto.guild_id,
            dto.previous_states,
            dto.duration_secs,
            dto.imposed_rate,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/security/slowmode/{guild_id} — bot retire un slowmode
/// (deactivation manuelle ou via worker).
pub async fn delete_slowmode(
    State(state): State<SystemState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<StatusCode, ApiError> {
    state.slowmode_uc.deactivate(&guild_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
