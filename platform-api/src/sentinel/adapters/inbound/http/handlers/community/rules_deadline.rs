//! Delai d'acceptation du reglement : ouverture et fermeture du compte a
//! rebours d'un arrivant.
//!
//! Appele par le bot, seul a voir les arrivees et les clics. Le bot n'envoie
//! jamais de duree : c'est l'API qui lit le reglage de la guilde et calcule
//! l'echeance — sinon le delai vivrait a deux endroits, et le message annoncant
//! « trois jours » finirait par mentir.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::bootstrap::state::CommunityState;

#[derive(Debug, Deserialize)]
pub struct RulesDeadlineDto {
    pub guild_id: String,
    pub user_id: String,
}

/// Ce que le serveur a applique, pour que le bot annonce le vrai delai.
#[derive(Debug, Serialize)]
pub struct RulesDeadlineAppliedDto {
    pub enabled: bool,
    pub deadline_secs: i64,
    pub reminder_secs: i64,
    pub kick_enabled: bool,
}

/// POST /api/community/rules-deadline/start
pub async fn start_rules_deadline(
    State(state): State<CommunityState>,
    Json(dto): Json<RulesDeadlineDto>,
) -> Result<Json<RulesDeadlineAppliedDto>, ApiError> {
    let applique = state
        .rules_deadline_uc
        .start(&dto.guild_id, &dto.user_id)
        .await?;
    Ok(Json(RulesDeadlineAppliedDto {
        enabled: applique.enabled,
        deadline_secs: applique.deadline_secs,
        reminder_secs: applique.reminder_secs,
        kick_enabled: applique.kick_enabled,
    }))
}

/// POST /api/community/rules-deadline/clear
///
/// Un POST plutot qu'un DELETE : le bot n'emet plus aucun DELETE HTTP, tous
/// passes en gRPC ou en fire-and-forget.
pub async fn clear_rules_deadline(
    State(state): State<CommunityState>,
    Json(dto): Json<RulesDeadlineDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .rules_deadline_uc
        .clear(&dto.guild_id, &dto.user_id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
