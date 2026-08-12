use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::system::tickets::AssignDto;
use crate::adapters::inbound::http::dto::system::tickets::CreateTicketDto;
use crate::adapters::inbound::http::dto::system::tickets::ListTicketsQuery;
use crate::adapters::inbound::http::dto::system::tickets::ReplyDto;
use crate::adapters::inbound::http::dto::system::tickets::TicketDetailDto;
use crate::adapters::inbound::http::dto::system::tickets::TicketResponseDto;
use crate::adapters::inbound::http::dto::system::tickets::UpdateStatusDto;
use crate::adapters::inbound::http::dto::system::tickets::UpdateTicketChannelDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::adapters::inbound::http::validation;
use crate::bootstrap::state::SystemState;
use sentinel_core::domain::enums::system::ticket_status::TicketStatus;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::inbound::system::manage_tickets::AssignTicketCommand;
use sentinel_core::ports::inbound::system::manage_tickets::ReplyTicketCommand;
use sentinel_core::ports::inbound::system::manage_tickets::UpdateTicketChannelCommand;

pub async fn list_tickets(
    State(state): State<SystemState>,
    _user: Option<Extension<WebUser>>,
    Query(params): Query<ListTicketsQuery>,
) -> Result<Json<Vec<TicketResponseDto>>, ApiError> {
    // Validation
    validation::validate_pagination(params.limit, params.offset).map_err(ApiError)?;
    validation::validate_search(&params.search).map_err(ApiError)?;

    let limit = crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 200);
    let offset = crate::adapters::inbound::http::helpers::normalize_offset(params.offset);
    let tickets = state
        .tickets_uc
        .list_tickets(
            params.status,
            params.priority,
            params.search,
            params.author_id,
            limit,
            offset,
        )
        .await?;

    // Plus de scope par role : le back-office est superadmin-only, donc tout
    // appelant web qui arrive ici a deja ete autorise par `auth-api` via
    // `superadmin_middleware`. Le filtre precedent comparait l'identite au
    // `SUPERADMIN_USER_IDS` LOCAL puis, en cas de non-correspondance, retombait
    // sur `moderated_guilds` — qui interroge `api_user_guilds`, table supprimee
    // par la migration 007. Le moindre ecart entre la liste locale et celle de
    // l'identite transformait donc cet ecran en 500.
    Ok(map_to_dtos(tickets))
}

pub async fn get_ticket_detail(
    State(state): State<SystemState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<String>,
) -> Result<Json<TicketDetailDto>, ApiError> {
    let detail = state.tickets_uc.get_ticket_detail(&id).await?;
    Ok(single_dto(detail))
}

pub async fn create_ticket(
    State(state): State<SystemState>,
    user: Option<Extension<WebUser>>,
    Json(dto): Json<CreateTicketDto>,
) -> Result<Json<TicketResponseDto>, ApiError> {
    // Validation
    validation::validate_title(&dto.title).map_err(ApiError)?;

    let mut command: sentinel_core::ports::inbound::system::manage_tickets::CreateTicketCommand =
        dto.into();

    // S1/S4 — chemin web : la creation HTTP exige Moderator+ sur la guild cible,
    // et l'auteur est DERIVE du principal authentifie (on n'autorise pas un
    // `author_id` arbitraire dans le body -> anti-impersonation). Le chemin
    // bot/interne (gRPC, qui pose legitimement author = l'utilisateur Discord)
    // reste inchange.
    if let Some(Extension(ctx)) = user.as_ref() {
        let Some(_gid) = command.guild_id.clone() else {
            return Err(ApiError(DomainError::Forbidden(
                "guild_id requis pour creer un ticket via le web".into(),
            )));
        };
        command.author_id = ctx.discord_user_id.clone();
    }

    let ticket = state.tickets_uc.create_ticket(command).await?;

    state.broadcaster.broadcast(
        "ticket_new",
        serde_json::json!({
            "id": ticket.id.to_string(),
            "title": &ticket.title,
            "author_name": &ticket.author_name,
            "priority": &ticket.priority,
        }),
    );

    Ok(single_dto(ticket))
}

pub async fn reply_ticket(
    State(state): State<SystemState>,
    user: Option<Extension<WebUser>>,
    Path(id): Path<String>,
    Json(dto): Json<ReplyDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // S4 identite : pour un appelant web on derive `author_name`/`author_role`
    // du principal REEL, le body est ignore pour ces champs — impossible de se
    // faire passer pour quelqu'un d'autre via un JSON forge. Un appelant web
    // est necessairement superadmin (gate en amont), d'ou le role "admin".
    // Bot/interne : on garde les valeurs du body (vraies perms Discord).
    let (author_name, author_role) = match user.as_ref() {
        None => (dto.author_name, dto.author_role),
        Some(Extension(u)) => (u.discord_user_id.clone(), "admin".to_string()),
    };

    let broadcast_name = author_name.clone();

    state
        .tickets_uc
        .reply_ticket(ReplyTicketCommand {
            ticket_id: id.clone(),
            content: dto.content,
            author_name,
            author_role,
        })
        .await?;

    state.broadcaster.broadcast(
        "ticket_message",
        serde_json::json!({
            "ticket_id": &id,
            "author_name": &broadcast_name,
        }),
    );

    Ok(ok_response())
}

pub async fn close_ticket(
    State(state): State<SystemState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.tickets_uc.close_ticket(&id).await?;

    // Phase 2 sync : enrichi avec `action_id` (= ticket_id parse en UUID)
    // pour que le bot puisse retrouver le mapping discord_action_messages
    // et lock le channel Discord. Format aligne sur SYNC_DISCORD_WEB_DESIGN.md.
    let action_id = uuid::Uuid::parse_str(&id).ok();
    state.broadcaster.broadcast(
        "ticket_closed",
        serde_json::json!({
            "ticket_id": &id,
            "action_id": action_id,
            "actor": { "source": "web" },
        }),
    );

    Ok(ok_response())
}

pub async fn assign_ticket(
    State(state): State<SystemState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<String>,
    Json(dto): Json<AssignDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let assignee = dto.assignee;
    state
        .tickets_uc
        .assign_ticket(AssignTicketCommand {
            ticket_id: id.clone(),
            assignee: assignee.clone(),
        })
        .await?;

    state.broadcaster.broadcast(
        "ticket_assigned",
        serde_json::json!({
            "ticket_id": &id,
            "assignee": &assignee,
        }),
    );

    Ok(ok_response())
}

pub async fn update_status(
    State(state): State<SystemState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateStatusDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let status = match TicketStatus::from_str(&dto.status) {
        Some(s) => s,
        None => {
            return Err(DomainError::ValidationError(format!(
                "Statut invalide : {}. Valeurs acceptees : {:?}",
                dto.status,
                TicketStatus::VALID_VALUES
            ))
            .into())
        }
    };

    if status == TicketStatus::Closed {
        state.tickets_uc.close_ticket(&id).await?;
    } else {
        state.tickets_uc.update_status(&id, &dto.status).await?;
    }

    state.broadcaster.broadcast(
        "ticket_status_updated",
        serde_json::json!({ "ticket_id": &id, "status": &dto.status }),
    );

    Ok(ok_response())
}

pub async fn update_ticket_channel(
    State(state): State<SystemState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateTicketChannelDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .tickets_uc
        .update_ticket_channel(UpdateTicketChannelCommand {
            ticket_id: id.clone(),
            voice_channel_id: dto.voice_channel_id,
            invited_user_id: dto.invited_user_id,
        })
        .await?;

    state.broadcaster.broadcast(
        "ticket_channel_updated",
        serde_json::json!({ "ticket_id": &id }),
    );

    Ok(ok_response())
}

#[derive(Debug, Deserialize)]
pub struct BulkDeleteTicketsParams {
    /// Filtrer par auteur (Discord user id). Optionnel.
    pub author_id: Option<String>,
    /// Borne inclusive de date (RFC3339). Optionnel.
    pub from: Option<String>,
    /// Borne inclusive de date (RFC3339). Optionnel.
    pub to: Option<String>,
    /// Safety : si true, permet de supprimer TOUS les tickets (pas de filtre).
    /// Sinon au moins un filtre est requis pour eviter un DELETE sans bornes
    /// par accident.
    #[serde(default)]
    pub all: bool,
}

/// DELETE /api/tickets/bulk — suppression en masse avec filtres optionnels.
///
/// Filtres combinables (AND) :
/// - `author_id` : ne supprime que les tickets crees par ce user
/// - `from` / `to` : plage de dates (inclusive), format RFC3339 ou YYYY-MM-DD
/// - `all=true` : autorise la suppression totale si aucun filtre fourni
///
/// Utilise un CTE pour supprimer en premier les `ticket_messages` lies
/// (meme si ON DELETE CASCADE est en place — on reste explicite pour
/// pouvoir compter ce qui a ete supprime sans joindre).
///
/// Controle d'acces : `superadmin_middleware`, pose au niveau du routeur. Il n'y
/// a pas de gate propre a ce handler — l'ancien calculait `is_superadmin` puis
/// n'en faisait rien (`if !is_superadmin {}`), reste d'un `require_role` retire
/// avec le RBAC multi-roles.
pub async fn bulk_delete_tickets(
    State(state): State<SystemState>,
    _user: Option<Extension<WebUser>>,
    Query(params): Query<BulkDeleteTicketsParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let has_filter = params.author_id.is_some() || params.from.is_some() || params.to.is_some();
    if !has_filter && !params.all {
        return Err(ApiError(DomainError::ValidationError(
            "Aucun filtre fourni. Passe all=true pour supprimer TOUS les tickets.".into(),
        )));
    }

    // Parse optionnel des dates (RFC3339 ou YYYY-MM-DD → UTC minuit).
    fn parse_date(s: &str, end_of_day: bool) -> Result<chrono::DateTime<chrono::Utc>, DomainError> {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Ok(dt.with_timezone(&chrono::Utc));
        }
        if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            let time = if end_of_day {
                chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap()
            } else {
                chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
            };
            return Ok(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                d.and_time(time),
                chrono::Utc,
            ));
        }
        Err(DomainError::ValidationError(format!(
            "Date invalide '{s}' (attendu RFC3339 ou YYYY-MM-DD)"
        )))
    }

    let from_dt = params
        .from
        .as_deref()
        .map(|s| parse_date(s, false))
        .transpose()
        .map_err(ApiError)?;
    let to_dt = params
        .to
        .as_deref()
        .map(|s| parse_date(s, true))
        .transpose()
        .map_err(ApiError)?;

    let deleted = state
        .tickets_uc
        .bulk_delete_tickets(params.author_id.as_deref(), from_dt, to_dt)
        .await?;
    tracing::info!(
        deleted,
        author_id = ?params.author_id,
        from = ?params.from,
        to = ?params.to,
        all = params.all,
        "bulk_delete_tickets"
    );

    Ok(Json(serde_json::json!({
        "deleted": deleted,
        "author_id": params.author_id,
        "from": params.from,
        "to": params.to,
    })))
}

#[cfg(test)]
#[path = "tests/tickets.rs"]
mod tests;
