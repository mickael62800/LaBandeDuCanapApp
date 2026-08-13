//! Sondages : lecture publique des resultats, vote authentifie.
//!
//! Les pourcentages sont calcules cote domaine et transmis tels quels : le
//! site et le bot doivent afficher exactement les memes chiffres, ils ne
//! peuvent pas arrondir chacun de leur cote.

use axum::extract::{Path, Query, State};
use axum::Extension;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::handlers::community::public_guard::{
    clamp_limit, ensure_guild_id,
};
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::bootstrap::state::CommunityState;
use platform_core::sentinel::domain::entities::community::poll::{Poll, UpsertPollCommand};
use platform_core::sentinel::domain::errors::DomainError;

const DEFAULT_LIMIT: i64 = 10;
const MAX_LIMIT: i64 = 30;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    /// Back-office : inclure les sondages clos.
    #[serde(
        default,
        deserialize_with = "crate::sentinel::adapters::inbound::http::helpers::bool_souple"
    )]
    pub all: bool,
}

// ── DTO ──

#[derive(Debug, Serialize)]
pub struct PollOptionDto {
    pub id: Uuid,
    pub label: String,
    /// Toujours renseignee : la palette de repli est appliquee ici, pour que
    /// le front n'ait pas a dupliquer la meme suite de couleurs.
    pub color: String,
    pub votes: i64,
    pub share: i32,
}

#[derive(Debug, Serialize)]
pub struct PollDto {
    pub id: Uuid,
    pub question: String,
    pub description: Option<String>,
    pub closes_at: String,
    pub is_closed: bool,
    pub is_open: bool,
    pub total_votes: i64,
    pub options: Vec<PollOptionDto>,
    /// Option choisie par le lecteur, si connecte.
    pub my_vote: Option<Uuid>,
}

/// Construit le DTO en appliquant palette et pourcentages du domaine.
fn to_dto(poll: Poll, my_vote: Option<Uuid>) -> PollDto {
    let is_open = poll.is_open(chrono::Utc::now());
    let total_votes = poll.total_votes();
    let shares = poll.shares();
    let colors: Vec<String> = (0..poll.options.len()).map(|i| poll.color_at(i)).collect();

    PollDto {
        id: poll.id,
        question: poll.question,
        description: poll.description,
        closes_at: poll.closes_at.to_rfc3339(),
        is_closed: poll.is_closed,
        is_open,
        total_votes,
        options: poll
            .options
            .into_iter()
            .zip(shares)
            .zip(colors)
            .map(|((o, share), color)| PollOptionDto {
                id: o.id,
                label: o.label,
                color,
                votes: o.votes,
                share,
            })
            .collect(),
        my_vote,
    }
}

#[derive(Debug, Deserialize)]
pub struct CreatePollDto {
    pub question: String,
    pub description: Option<String>,
    /// RFC3339.
    pub closes_at: String,
    #[serde(default = "default_true")]
    pub is_public: bool,
    pub options: Vec<CreateOptionDto>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOptionDto {
    pub label: String,
    pub color: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct VoteDto {
    pub option_id: Uuid,
}

// ── Back-office ──

/// GET /api/polls/{guild_id}
pub async fn list_polls(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<PollDto>>, ApiError> {
    let polls = state
        .polls_uc
        .list(
            &guild_id,
            !q.all,
            clamp_limit(q.limit, DEFAULT_LIMIT, MAX_LIMIT),
        )
        .await?;
    Ok(Json(polls.into_iter().map(|p| to_dto(p, None)).collect()))
}

/// POST /api/polls/{guild_id}
pub async fn create_poll(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Json(dto): Json<CreatePollDto>,
) -> Result<Json<PollDto>, ApiError> {
    let closes_at = chrono::DateTime::parse_from_rfc3339(&dto.closes_at)
        .map(|d| d.with_timezone(&chrono::Utc))
        .map_err(|_| {
            ApiError(DomainError::ValidationError(
                "closes_at invalide (RFC3339)".into(),
            ))
        })?;

    let cmd = UpsertPollCommand {
        guild_id,
        question: dto.question,
        description: dto.description,
        closes_at,
        is_public: dto.is_public,
        created_by: user
            .as_ref()
            .map(|Extension(c)| c.discord_user_id.clone())
            .unwrap_or_default(),
        options: dto
            .options
            .into_iter()
            .map(|o| (o.label, o.color))
            .collect(),
    };
    Ok(Json(to_dto(state.polls_uc.create(cmd).await?, None)))
}

/// POST /api/polls/detail/{id}/close
pub async fn close_poll(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.polls_uc.get(id, None).await?;

    state.polls_uc.close(id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/polls/detail/{id}
pub async fn delete_poll(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.polls_uc.get(id, None).await?;

    state.polls_uc.delete(id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// POST /api/polls/detail/{id}/vote
pub async fn vote_poll(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<VoteDto>,
) -> Result<Json<PollDto>, ApiError> {
    let Some(Extension(ctx)) = user else {
        return Err(ApiError(DomainError::Forbidden(
            "auth Discord requise pour voter".into(),
        )));
    };

    let poll = state
        .polls_uc
        .vote(id, dto.option_id, &ctx.discord_user_id)
        .await?;
    Ok(Json(to_dto(poll, Some(dto.option_id))))
}

/// GET /api/me/polls/{guild_id}
///
/// Sondages ouverts, avec le vote du LECTEUR pre-coche.
///
/// Distinct de `list_polls`, qui exige `Viewer` : un membre ordinaire n'a pas
/// ce role et se verrait refuser sa propre page. Distinct aussi de
/// `public_polls`, qui ne connait aucune identite et ne peut donc pas dire
/// pour quoi on a vote.
///
/// Aucun parametre `all` ici : un membre n'a pas a fouiller les archives.
pub async fn my_polls(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<PollDto>>, ApiError> {
    let Some(Extension(ctx)) = user else {
        return Err(ApiError(DomainError::Forbidden(
            "auth Discord requise".into(),
        )));
    };

    // L'appartenance a la guilde est deja verifiee par `guild_auth_middleware`
    // sur les routes portant `{guild_id}` : inutile de la revalider ici.
    let polls = state.polls_uc.list(&guild_id, true, DEFAULT_LIMIT).await?;

    // Le vote personnel se recupere sondage par sondage : le port de liste ne
    // le porte pas, et l'ajouter obligerait tous les appelants a fournir une
    // identite qu'ils n'ont pas.
    let mut out = Vec::with_capacity(polls.len());
    for p in polls {
        let mien = state
            .polls_uc
            .get(p.id, Some(&ctx.discord_user_id))
            .await
            .ok()
            .and_then(|v| v.my_vote);
        out.push(to_dto(p, mien));
    }
    Ok(Json(out))
}

// ── Surface PUBLIQUE ──

/// GET /api/public/polls/{guild_id}
///
/// Sondages ouverts uniquement. Le DTO est le meme que le back-office parce
/// qu'il ne contient deja aucune donnee sensible : ni auteur, ni qui a vote
/// quoi. `my_vote` reste `None`, un visiteur n'ayant pas d'identite.
pub async fn public_polls(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<PollDto>>, ApiError> {
    ensure_guild_id(&guild_id)?;

    // `open_only` force a true : `?all=1` ne doit pas exposer les archives.
    let polls = state
        .polls_uc
        .list(
            &guild_id,
            true,
            clamp_limit(q.limit, DEFAULT_LIMIT, MAX_LIMIT),
        )
        .await?;

    // Un sondage marque non public reste hors de la vue publique.
    Ok(Json(
        polls
            .into_iter()
            .filter(|p| p.is_public)
            .map(|p| to_dto(p, None))
            .collect(),
    ))
}
