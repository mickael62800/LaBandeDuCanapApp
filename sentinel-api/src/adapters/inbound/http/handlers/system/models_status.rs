use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use tracing::info;

use crate::bootstrap::state::AiState;
use sentinel_core::domain::entities::ai::ai_models::format_model_display_name;

#[derive(Debug, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub model_type: String,
    pub loaded: bool,
}

#[derive(Debug, Serialize)]
pub struct ModelsStatusResponse {
    pub models: Vec<ModelInfo>,
}

/// GET /api/models/status — retourne l'etat des modeles IA charges
pub async fn get_models_status(State(state): State<AiState>) -> Json<ModelsStatusResponse> {
    let vision_path = std::env::var("VISION_MODEL_PATH").unwrap_or_default();
    let text_path = std::env::var("TEXT_MODEL_PATH").unwrap_or_default();

    let models = vec![
        ModelInfo {
            name: format_model_display_name("Vision", &vision_path),
            model_type: "vision".to_string(),
            loaded: state.inference.vision_available(),
        },
        ModelInfo {
            name: format_model_display_name("Text", &text_path),
            model_type: "text".to_string(),
            loaded: state.inference.text_available(),
        },
    ];

    Json(ModelsStatusResponse { models })
}

#[derive(Debug, Deserialize)]
pub struct ReloadRequest {
    pub model_type: String,
}

#[derive(Debug, Serialize)]
pub struct ReloadResponse {
    pub success: bool,
    pub message: String,
}

/// POST /api/models/reload — recharge un modele ONNX a chaud
pub async fn reload_model(
    State(state): State<AiState>,
    Json(req): Json<ReloadRequest>,
) -> (StatusCode, Json<ReloadResponse>) {
    info!(model_type = %req.model_type, "Rechargement du modele demande");
    match state.inference.reload(&req.model_type) {
        Ok(msg) => {
            info!("{}", msg);
            (
                StatusCode::OK,
                Json(ReloadResponse {
                    success: true,
                    message: msg,
                }),
            )
        }
        Err(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ReloadResponse {
                success: false,
                message: msg,
            }),
        ),
    }
}

#[cfg(test)]
#[path = "tests/models_status.rs"]
mod tests;
