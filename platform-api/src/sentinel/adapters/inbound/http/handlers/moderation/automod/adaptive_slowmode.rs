//! Persistance des salons en slowmode adaptatif (BUG3) : le bot marque/retire
//! les salons a l'activation/desactivation et recharge l'ensemble au demarrage.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::bootstrap::state::ModerationState;

#[derive(Deserialize, Serialize)]
pub struct AdaptiveSlowmodeBody {
    #[serde(default)]
    pub guild_id: String,
    pub channel_id: String,
}

/// POST /api/automod/adaptive-slowmode — marque un salon comme actif.
pub async fn mark_adaptive_slowmode(
    State(state): State<ModerationState>,
    Json(body): Json<AdaptiveSlowmodeBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .automod_adaptive_slowmode_repo
        .mark(&body.guild_id, &body.channel_id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/automod/adaptive-slowmode/remove — retire un salon (desactive).
pub async fn unmark_adaptive_slowmode(
    State(state): State<ModerationState>,
    Json(body): Json<AdaptiveSlowmodeBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .automod_adaptive_slowmode_repo
        .unmark(&body.channel_id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /api/automod/adaptive-slowmode — liste tous les salons actifs (reload).
pub async fn list_adaptive_slowmode(
    State(state): State<ModerationState>,
) -> Result<Json<Vec<AdaptiveSlowmodeBody>>, ApiError> {
    let rows = state.automod_adaptive_slowmode_repo.list_all().await?;
    Ok(Json(
        rows.into_iter()
            .map(|(guild_id, channel_id)| AdaptiveSlowmodeBody {
                guild_id,
                channel_id,
            })
            .collect(),
    ))
}
