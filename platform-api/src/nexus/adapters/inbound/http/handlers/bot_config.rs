//! Lecture/ecriture de la config bot par guild (`bot_guild_config`).
//!
//! Consomme par le bot pour resoudre les reglages game-portal d'une guild
//! (categorie des salons de session, hote public, ping quotidien…).
//! Protection : Bearer global uniquement, comme le reste de /api.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use std::collections::HashMap;

use crate::nexus::adapters::inbound::http::handlers::ApiError;
use crate::nexus::bootstrap::AppState;
use platform_core::nexus::domain::entities::system::bot_config::BotDefinition;

/// GET /api/bots/definitions
///
/// Liste les modules configurables et leur schema (`config_schema`). Meme
/// forme que l'endpoint homonyme de sentinel-api, pour que le formulaire de
/// configuration generique du front fonctionne sans adaptation.
pub async fn get_definitions(
    State(state): State<AppState>,
) -> Result<Json<Vec<BotDefinition>>, ApiError> {
    Ok(Json(state.bot_config_repo.get_definitions().await?))
}

/// GET /api/config/{guild_id}/{bot_name}
///
/// Renvoie un objet plat `{ cle: valeur }` — format attendu par le bot.
pub async fn get_config(
    State(state): State<AppState>,
    Path((guild_id, bot_name)): Path<(String, String)>,
) -> Result<Json<HashMap<String, String>>, ApiError> {
    let entries = state
        .bot_config_repo
        .get_config(&guild_id, &bot_name)
        .await?;
    Ok(Json(
        entries
            .into_iter()
            .map(|e| (e.config_key, e.config_value))
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct SetConfigDto {
    pub key: String,
    pub value: String,
}

/// PUT /api/config/{guild_id}/{bot_name}
pub async fn set_config(
    State(state): State<AppState>,
    Path((guild_id, bot_name)): Path<(String, String)>,
    Json(dto): Json<SetConfigDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .bot_config_repo
        .set_config(&guild_id, &bot_name, &dto.key, &dto.value)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
