//! Planning communautaire : lecture, edition et inscriptions.
//!
//! Deux surfaces distinctes :
//!   - `/api/events/*` — reserve au back-office, protege par la pile
//!     d'authentification. L'ecriture exige `Moderator+`.
//!   - `/api/public/events/{guild_id}` — monte hors authentification (cf.
//!     `handlers::system::public_site`). Ne renvoie QUE les evenements publies
//!     et publics, et un DTO ecrit champ par champ : ni brouillon, ni auteur,
//!     ni liste d'inscrits.

use axum::extract::{Path, Query, State};
use axum::Extension;
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::bootstrap::state::CommunityState;
use platform_core::sentinel::domain::entities::community::event::{
    CommunityEvent, EventAnswer, EventStatus, UpsertEventCommand,
};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::event_repository::EventWindow;

/// Fenetre maximale interrogeable : un an. Au-dela, la requete ne sert aucune
/// vue reelle (semaine ou mois) et ne ferait que charger la base.
const MAX_WINDOW_DAYS: i64 = 366;

#[derive(Debug, Deserialize)]
pub struct WindowQuery {
    /// Bornes RFC3339. Absentes : le mois en cours.
    pub from: Option<String>,
    pub to: Option<String>,
}

fn parse_window(q: &WindowQuery) -> Result<EventWindow, ApiError> {
    let parse = |s: &Option<String>| {
        s.as_deref()
            .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
            .map(|d| d.with_timezone(&Utc))
    };

    let now = Utc::now();
    let from = parse(&q.from).unwrap_or(now - Duration::days(15));
    let to = parse(&q.to).unwrap_or(now + Duration::days(45));

    if to <= from {
        return Err(ApiError(DomainError::ValidationError(
            "fenetre invalide : `to` doit suivre `from`".into(),
        )));
    }
    if (to - from).num_days() > MAX_WINDOW_DAYS {
        return Err(ApiError(DomainError::ValidationError(
            "fenetre limitee a un an".into(),
        )));
    }
    Ok(EventWindow { from, to })
}

// ── DTO back-office ──

#[derive(Debug, Serialize)]
pub struct EventDto {
    pub id: Uuid,
    pub guild_id: String,
    pub title: String,
    pub description: Option<String>,
    pub game: Option<String>,
    pub color: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    pub all_day: bool,
    pub is_public: bool,
    pub status: String,
    /// Nombre de jours couverts : le calendrier dimensionne sa barre avec.
    pub span_days: i64,
    pub created_by: String,
    pub source_server_id: Option<Uuid>,
}

impl From<CommunityEvent> for EventDto {
    fn from(e: CommunityEvent) -> Self {
        let span_days = e.span_days();
        Self {
            id: e.id,
            guild_id: e.guild_id,
            title: e.title,
            description: e.description,
            game: e.game,
            color: e.color,
            starts_at: e.starts_at.to_rfc3339(),
            ends_at: e.ends_at.to_rfc3339(),
            all_day: e.all_day,
            is_public: e.is_public,
            status: e.status.as_str().to_string(),
            span_days,
            created_by: e.created_by,
            source_server_id: e.source_server_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ParticipantDto {
    pub user_id: String,
    pub username: String,
    pub answer: String,
}

#[derive(Debug, Serialize)]
pub struct EventDetailDto {
    pub event: EventDto,
    pub participants: Vec<ParticipantDto>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertEventDto {
    pub title: String,
    pub description: Option<String>,
    pub game: Option<String>,
    pub color: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    #[serde(default)]
    pub all_day: bool,
    #[serde(default = "default_true")]
    pub is_public: bool,
    #[serde(default)]
    pub status: Option<String>,
    /// Serveur de jeu Nexus a l'origine de cet evenement, s'il y en a un.
    /// Renseigne par la page de creation d'un serveur ; c'est ce qui permet de
    /// retrouver l'evenement pour le supprimer avec le serveur.
    #[serde(default)]
    pub source_server_id: Option<uuid::Uuid>,
}

fn default_true() -> bool {
    true
}

fn parse_dt(s: &str, field: &str) -> Result<DateTime<Utc>, ApiError> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| {
            ApiError(DomainError::ValidationError(format!(
                "{field} invalide (RFC3339)"
            )))
        })
}

// ── Back-office ──

/// GET /api/events/{guild_id}?from=&to=
pub async fn list_events(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Query(q): Query<WindowQuery>,
) -> Result<Json<Vec<EventDto>>, ApiError> {
    let events = state
        .events_uc
        .list_window(&guild_id, parse_window(&q)?, false)
        .await?;
    Ok(Json(events.into_iter().map(Into::into).collect()))
}

/// GET /api/events/detail/{id}
pub async fn get_event(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<EventDetailDto>, ApiError> {
    let detail = state.events_uc.get(id).await?;

    Ok(Json(EventDetailDto {
        participants: detail
            .participants
            .iter()
            .map(|p| ParticipantDto {
                user_id: p.user_id.clone(),
                username: p.username.clone(),
                answer: p.answer.as_str().to_string(),
            })
            .collect(),
        event: detail.event.into(),
    }))
}

/// POST /api/events/{guild_id}
pub async fn create_event(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Json(dto): Json<UpsertEventDto>,
) -> Result<Json<EventDto>, ApiError> {
    let author = user
        .as_ref()
        .map(|Extension(c)| c.discord_user_id.clone())
        .unwrap_or_default();

    let cmd = UpsertEventCommand {
        guild_id,
        title: dto.title,
        description: dto.description,
        game: dto.game,
        color: dto.color,
        starts_at: parse_dt(&dto.starts_at, "starts_at")?,
        ends_at: parse_dt(&dto.ends_at, "ends_at")?,
        all_day: dto.all_day,
        is_public: dto.is_public,
        status: dto
            .status
            .as_deref()
            .map(EventStatus::parse)
            .unwrap_or(EventStatus::Published),
        created_by: author,
        source_server_id: dto.source_server_id,
    };
    Ok(Json(state.events_uc.create(cmd).await?.into()))
}

/// PUT /api/events/detail/{id}
pub async fn update_event(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpsertEventDto>,
) -> Result<Json<EventDto>, ApiError> {
    let existing = state.events_uc.get(id).await?;

    let cmd = UpsertEventCommand {
        guild_id: existing.event.guild_id.clone(),
        title: dto.title,
        description: dto.description,
        game: dto.game,
        color: dto.color,
        starts_at: parse_dt(&dto.starts_at, "starts_at")?,
        ends_at: parse_dt(&dto.ends_at, "ends_at")?,
        all_day: dto.all_day,
        is_public: dto.is_public,
        status: dto
            .status
            .as_deref()
            .map(EventStatus::parse)
            .unwrap_or(EventStatus::Published),
        created_by: existing.event.created_by,
        // Conserve tel quel : le rattachement est pose a la creation par la
        // page de creation d'un serveur. Le laisser modifier depuis une edition
        // d'evenement permettrait de faire disparaitre un serveur du calendrier
        // en detachant sa soiree, ou d'en accrocher une a un serveur tiers.
        source_server_id: existing.event.source_server_id,
    };
    Ok(Json(state.events_uc.update(id, cmd).await?.into()))
}

/// DELETE /api/events/detail/{id}
pub async fn delete_event(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.events_uc.get(id).await?;

    state.events_uc.delete(id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[derive(Debug, Deserialize)]
pub struct JoinDto {
    #[serde(default)]
    pub answer: Option<String>,
}

/// POST /api/events/detail/{id}/join — s'inscrire (ou changer d'avis).
pub async fn join_event(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<JoinDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(Extension(ctx)) = user else {
        return Err(ApiError(DomainError::Forbidden(
            "auth Discord requise".into(),
        )));
    };

    state
        .events_uc
        .join(
            id,
            &ctx.discord_user_id,
            "",
            dto.answer
                .as_deref()
                .map(EventAnswer::parse)
                .unwrap_or(EventAnswer::Going),
        )
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/events/detail/{id}/join — se desinscrire.
pub async fn leave_event(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(Extension(ctx)) = user else {
        return Err(ApiError(DomainError::Forbidden(
            "auth Discord requise".into(),
        )));
    };
    state.events_uc.leave(id, &ctx.discord_user_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Surface PUBLIQUE ──

/// Vue publique d'un evenement. DTO distinct, ecrit champ par champ : il ne
/// doit jamais suivre l'entite du domaine, sinon la premiere colonne ajoutee
/// (auteur, notes internes) se retrouverait publiee sur Internet.
#[derive(Debug, Serialize)]
pub struct PublicEventDto {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub game: Option<String>,
    pub color: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    pub all_day: bool,
    pub span_days: i64,
}

/// GET /api/public/events/{guild_id}?from=&to=
pub async fn public_events(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Query(q): Query<WindowQuery>,
) -> Result<Json<Vec<PublicEventDto>>, ApiError> {
    // Endpoint non authentifie : validation stricte, il est expose au balayage.
    if guild_id.len() > 20 || !guild_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError(DomainError::ValidationError(
            "guild_id invalide".into(),
        )));
    }

    let events = state
        .events_uc
        .list_window(&guild_id, parse_window(&q)?, true)
        .await?;

    Ok(Json(
        events
            .into_iter()
            .map(|e| PublicEventDto {
                // Calcule AVANT de deplacer les champs.
                span_days: e.span_days(),
                id: e.id,
                title: e.title,
                description: e.description,
                game: e.game,
                color: e.color,
                starts_at: e.starts_at.to_rfc3339(),
                ends_at: e.ends_at.to_rfc3339(),
                all_day: e.all_day,
            })
            .collect(),
    ))
}
