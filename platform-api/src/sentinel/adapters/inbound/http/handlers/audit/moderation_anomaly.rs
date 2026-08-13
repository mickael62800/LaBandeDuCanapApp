use axum::extract::State;
use axum::Json;

use crate::sentinel::adapters::inbound::http::dto::audit::moderation_anomaly::DetectAnomalyRequestDto;
use crate::sentinel::adapters::inbound::http::dto::audit::moderation_anomaly::DetectAnomalyResponseDto;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::bootstrap::state::AuditState;

/// POST /api/moderation-anomaly — le bot envoie un evenement de moderation ;
/// l'API agrege (fenetre glissante serveur), decide s'il y a anomalie et
/// renvoie l'alerte a afficher le cas echeant. La DECISION est server-side.
pub async fn detect_moderation_anomaly(
    State(state): State<AuditState>,
    Json(dto): Json<DetectAnomalyRequestDto>,
) -> Result<Json<DetectAnomalyResponseDto>, ApiError> {
    let alert = state.detect_anomaly_uc.detect(dto.into()).await;
    Ok(Json(DetectAnomalyResponseDto {
        alert: alert.map(Into::into),
    }))
}
