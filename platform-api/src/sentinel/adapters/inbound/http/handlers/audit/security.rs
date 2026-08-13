use crate::sentinel::adapters::inbound::http::dto::audit::security::ReportEventDto;
use crate::sentinel::adapters::inbound::http::dto::audit::security::SecurityEventResponseDto;
use crate::sentinel::adapters::inbound::http::dto::audit::security::SecurityQueryParams;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use crate::sentinel::adapters::inbound::http::helpers::map_to_dtos;
use crate::sentinel::adapters::inbound::http::helpers::single_dto;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::adapters::inbound::http::validation;
use crate::sentinel::bootstrap::state::AuditState;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;

/// POST /api/security/events — signaler un événement de sécurité (depuis le security-bot)
pub async fn report_event(
    State(state): State<AuditState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<ReportEventDto>,
) -> Result<Json<SecurityEventResponseDto>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_reason(&dto.description).map_err(ApiError)?;
    // Reserve au security-bot (Bearer API_KEY -> Internal, bypass) et aux admins
    // du serveur concerne. Empeche un user web de forger des evenements de
    // securite (faux positifs / watched users) pour une guilde arbitraire.

    let (command, (event_type, severity, description, guild_id)) =
        crate::capture_and_into!(dto, event_type, severity, description, guild_id);
    let event = state.security_uc.report_event(command).await?;

    // Broadcast WebSocket pour l'app desktop
    state.broadcaster.broadcast(
        "security_event",
        serde_json::json!({
            "guild_id": guild_id,
            "event_type": event_type,
            "severity": severity,
            "description": description,
        }),
    );

    Ok(single_dto(event))
}

/// DELETE /api/security/events/{guild_id}
/// Purge tous les evenements de securite d'une guild + les manual_watched_users
/// crees automatiquement par ces evenements.
pub async fn purge_events(
    State(state): State<AuditState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<serde_json::Value>, ApiError> {
    // La purge (SQL) vit dans le use case / repo (plus de SQL inline).
    let (deleted_events, deleted_watches) = state.security_uc.purge_events(&guild_id).await?;

    Ok(Json(serde_json::json!({
        "deleted_events": deleted_events,
        "deleted_watches": deleted_watches,
    })))
}

/// GET /api/security/events — lister les événements de sécurité
pub async fn list_events(
    State(state): State<AuditState>,
    _user: Option<Extension<WebUser>>,
    Query(params): Query<SecurityQueryParams>,
) -> Result<Json<Vec<SecurityEventResponseDto>>, ApiError> {
    let events = state
        .security_uc
        .list_events(params.guild_id.as_deref())
        .await?;

    Ok(map_to_dtos(events))
}
