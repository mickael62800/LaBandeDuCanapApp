use crate::sentinel::adapters::inbound::http::dto::audit::audit_logs::AuditLogQueryParams;
use crate::sentinel::adapters::inbound::http::dto::audit::audit_logs::AuditLogResponseDto;
use crate::sentinel::adapters::inbound::http::dto::audit::audit_logs::CreateAuditLogDto;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use crate::sentinel::adapters::inbound::http::helpers::map_to_dtos;
use crate::sentinel::adapters::inbound::http::helpers::normalize_limit;
use crate::sentinel::adapters::inbound::http::helpers::normalize_offset;
use crate::sentinel::adapters::inbound::http::helpers::single_dto;
use crate::sentinel::bootstrap::state::AuditState;
use axum::extract::Query;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::audit::manage_audit_logs::AuditLogFilters;

pub async fn create_audit_log(
    State(state): State<AuditState>,
    Json(dto): Json<CreateAuditLogDto>,
) -> Result<Json<AuditLogResponseDto>, ApiError> {
    let log = state.audit_logs_uc.create(dto.into()).await?;
    let response = single_dto(log);

    // Diffusion temps reel. Sans elle, la page Audit ne se rafraichissait que
    // "par accident", quand un log texte (POST /api/logs) passait en parallele
    // et declenchait `log_entry_created`. Indispensable des lors que le web
    // remplace les salons de logs Discord : un evenement manque n'a plus
    // aucune autre trace visible.
    state.broadcaster.broadcast(
        "audit_log_created",
        serde_json::to_value(&response.0).unwrap_or_default(),
    );

    Ok(response)
}

/// DELETE /api/audit-logs/{guild_id} — purge les audit logs d'une guild
/// anterieurs a 0 jours (= tout). Passe par le use case, pas de SQL direct.
pub async fn purge_audit_logs(
    State(state): State<AuditState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deleted = state
        .audit_logs_uc
        .delete_older_than_days(&guild_id, 0)
        .await?;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

pub async fn list_audit_logs(
    State(state): State<AuditState>,
    Query(params): Query<AuditLogQueryParams>,
) -> Result<axum::response::Response, ApiError> {
    // Securite : guild_id obligatoire pour eviter une fuite inter-guild.
    let guild_id = params.guild_id.ok_or_else(|| {
        ApiError(DomainError::ValidationError(
            "guild_id est obligatoire".into(),
        ))
    })?;

    // IDOR : le gate global ne protege JAMAIS les GET -> sans cette garde, tout
    // appelant lisait les audit logs (qui a banni/mute qui) de n'importe quel
    // serveur en changeant guild_id. Reserve moderator+ scope guilde.

    // Une date illisible est ignoree plutot que rejetee : un filtre mal forme
    // ne doit pas rendre le journal inaccessible.
    let parse_date = |v: Option<String>| {
        v.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&chrono::Utc))
    };

    let filters = AuditLogFilters {
        event_type: params.event_type,
        event_types: params
            .event_types
            .map(|raw| {
                raw.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default(),
        actor_id: params.actor_id,
        target_id: params.target_id,
        from: parse_date(params.from),
        to: parse_date(params.to),
        search: params
            .search
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        limit: normalize_limit(params.limit, 100, 500),
        offset: normalize_offset(params.offset),
    };

    // Le total part en en-tete plutot que dans le corps : la reponse reste un
    // tableau JSON, donc les clients existants ne cassent pas.
    let total = state.audit_logs_uc.count(Some(&guild_id), &filters).await?;
    let logs = state.audit_logs_uc.list(Some(&guild_id), filters).await?;

    let mut response = map_to_dtos::<_, AuditLogResponseDto>(logs).into_response();
    if let Ok(value) = axum::http::HeaderValue::from_str(&total.to_string()) {
        response.headers_mut().insert("X-Total-Count", value);
    }
    Ok(response)
}
