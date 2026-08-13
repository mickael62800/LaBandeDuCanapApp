use axum::extract::State;
use axum::Json;
use base64::Engine;

use crate::sentinel::adapters::inbound::http::dto::ai::analyze_image::AnalyzeImageRequestDto;
use crate::sentinel::adapters::inbound::http::dto::ai::analyze_image::AnalyzeImageResponseDto;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::bootstrap::state::AiState;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::ai::analyze_image::AnalyzeImageCommand;

// Limites et content-types autorises sont definis dans `domain/entities/image_analysis.rs`.
use platform_core::sentinel::domain::entities::ai::image_analysis::is_allowed_image_content_type;
use platform_core::sentinel::domain::entities::ai::image_analysis::is_image_size_acceptable;
use platform_core::sentinel::domain::entities::ai::image_analysis::MAX_IMAGE_BASE64_LEN;
pub async fn analyze_image(
    State(state): State<AiState>,
    Json(dto): Json<AnalyzeImageRequestDto>,
) -> Result<Json<AnalyzeImageResponseDto>, ApiError> {
    // Validation taille — regle metier dans `domain/entities/image_analysis.rs`.
    if !is_image_size_acceptable(dto.image_data.len()) {
        tracing::warn!(
            size = dto.image_data.len(),
            user_id = %dto.user_id,
            "Image trop volumineuse rejetee"
        );
        return Err(ApiError(DomainError::ValidationError(format!(
            "Image trop volumineuse ({} octets, max {})",
            dto.image_data.len(),
            MAX_IMAGE_BASE64_LEN
        ))));
    }

    // Validation content_type — regle metier dans le domain.
    if !is_allowed_image_content_type(&dto.content_type) {
        tracing::warn!(
            content_type = %dto.content_type,
            user_id = %dto.user_id,
            "Content-type image non autorise"
        );
        return Err(ApiError(DomainError::ValidationError(format!(
            "Content-type non autorise: {}",
            dto.content_type
        ))));
    }

    // Decoder le base64
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(&dto.image_data)
        .map_err(|e| {
            ApiError(DomainError::ValidationError(format!(
                "Base64 invalide: {e}"
            )))
        })?;

    let username = dto.username.clone();

    let command = AnalyzeImageCommand {
        guild_id: dto.guild_id,
        channel_id: dto.channel_id,
        user_id: dto.user_id,
        username: dto.username,
        message_id: dto.message_id,
        image_bytes,
        content_type: dto.content_type,
        filename: dto.filename,
    };

    let analysis = state.analyze_image_uc.analyze_image(command).await?;

    // Broadcast si action prise
    let action_str = analysis.action.as_str();
    if action_str != "none" {
        state.broadcaster.broadcast(
            "infraction_new",
            serde_json::json!({
                "username": username,
                "action": action_str,
                "reason": &analysis.reason,
                "type": "image",
            }),
        );
    }

    Ok(Json(AnalyzeImageResponseDto::from(analysis)))
}
