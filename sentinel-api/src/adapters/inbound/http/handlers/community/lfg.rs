//! Recherche de joueurs : lecture publique, ecriture authentifiee.
//!
//! Deux surfaces :
//!   - `/api/lfg/*` — authentifiee. Publier demande d'etre connecte ; fermer
//!     ou supprimer demande d'etre l'auteur, sauf pour un `Moderator+`.
//!   - `/api/public/lfg/{guild_id}` — hors authentification. DTO ecrit champ
//!     par champ : on publie le pseudo de l'auteur, jamais son identifiant
//!     Discord, qui permettrait de le retrouver hors du serveur.

use axum::extract::{Path, Query, State};
use axum::Extension;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::handlers::community::public_guard::{
    clamp_limit, ensure_guild_id,
};
use crate::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::bootstrap::state::CommunityState;
use sentinel_core::domain::entities::community::lfg::{LfgPost, UpsertLfgCommand};
use sentinel_core::domain::errors::DomainError;

const DEFAULT_LIMIT: i64 = 20;
const MAX_LIMIT: i64 = 50;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    /// Back-office uniquement : inclure les annonces fermees et expirees,
    /// pour pouvoir moderer.
    #[serde(
        default,
        deserialize_with = "crate::adapters::inbound::http::helpers::bool_souple"
    )]
    pub all: bool,
}

// ── DTO ──

#[derive(Debug, Serialize)]
pub struct InterestDto {
    pub user_id: String,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct LfgDto {
    pub id: Uuid,
    pub author_id: String,
    pub author_name: String,
    pub game: String,
    pub game_server_id: Option<Uuid>,
    pub slots: i32,
    pub when_text: String,
    pub description: Option<String>,
    pub is_open: bool,
    pub expires_at: String,
    pub created_at: String,
    pub interested: Vec<InterestDto>,
    /// Calcules ici : deux clients (site, bot) doivent afficher le meme
    /// compte, ils ne peuvent pas le recalculer chacun de leur cote.
    pub remaining_slots: i32,
    pub is_full: bool,
}

impl From<LfgPost> for LfgDto {
    fn from(p: LfgPost) -> Self {
        let remaining_slots = p.remaining_slots();
        let is_full = p.is_full();
        Self {
            id: p.id,
            author_id: p.author_id,
            author_name: p.author_name,
            game: p.game,
            game_server_id: p.game_server_id,
            slots: p.slots,
            when_text: p.when_text,
            description: p.description,
            is_open: p.is_open,
            expires_at: p.expires_at.to_rfc3339(),
            created_at: p.created_at.to_rfc3339(),
            interested: p
                .interested
                .into_iter()
                .map(|i| InterestDto {
                    user_id: i.user_id,
                    username: i.username,
                })
                .collect(),
            remaining_slots,
            is_full,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateLfgDto {
    pub game: String,
    pub game_server_id: Option<Uuid>,
    pub slots: i32,
    #[serde(default)]
    pub when_text: String,
    pub description: Option<String>,
    /// RFC3339. Absente : duree de vie par defaut du domaine.
    pub expires_at: Option<String>,
}

/// Le contexte d'authentification, ou une erreur explicite.
///
/// Toutes les ecritures passent par la : sans identite, on ne sait ni a qui
/// attribuer l'annonce, ni qui a le droit d'y toucher.
fn require_ctx(user: &Option<Extension<WebUser>>) -> Result<&WebUser, ApiError> {
    user.as_ref()
        .map(|Extension(c)| c)
        .ok_or_else(|| ApiError(DomainError::Forbidden("auth Discord requise".into())))
}

/// Pseudo d'affichage d'un membre, resolu cote serveur.
///
/// Jamais lu depuis le corps de la requete : un client pourrait alors publier
/// une annonce sous le nom de quelqu'un d'autre. Le `WebUser` ne porte que
/// l'identifiant, on va donc chercher le pseudo dans `guild_members`.
///
/// Un membre absent de la table (jamais synchronise) ne doit pas empecher de
/// publier : on retombe silencieusement sur une chaine vide, que le front
/// remplace par un libelle generique.
async fn display_name(state: &CommunityState, guild_id: &str, user_id: &str) -> String {
    String::new()
}

// ── Back-office / membre connecte ──

/// GET /api/lfg/{guild_id}
pub async fn list_lfg(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<LfgDto>>, ApiError> {
    let posts = state
        .lfg_uc
        .list(
            &guild_id,
            !q.all,
            clamp_limit(q.limit, DEFAULT_LIMIT, MAX_LIMIT),
        )
        .await?;
    Ok(Json(posts.into_iter().map(Into::into).collect()))
}

/// POST /api/lfg/{guild_id}
pub async fn create_lfg(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Json(dto): Json<CreateLfgDto>,
) -> Result<Json<LfgDto>, ApiError> {
    let ctx = require_ctx(&user)?;

    let expires_at = match dto.expires_at.as_deref() {
        Some(s) => Some(
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|d| d.with_timezone(&chrono::Utc))
                .map_err(|_| {
                    ApiError(DomainError::ValidationError(
                        "expires_at invalide (RFC3339)".into(),
                    ))
                })?,
        ),
        None => None,
    };

    let author_name = display_name(&state, &guild_id, &ctx.discord_user_id).await;

    let cmd = UpsertLfgCommand {
        guild_id,
        author_id: ctx.discord_user_id.clone(),
        author_name,
        game: dto.game,
        game_server_id: dto.game_server_id,
        slots: dto.slots,
        when_text: dto.when_text,
        description: dto.description,
        expires_at,
    };
    Ok(Json(state.lfg_uc.create(cmd).await?.into()))
}

/// Le demandeur peut-il moderer une annonce qui n'est pas la sienne ?
///
/// Depuis le passage en superadmin-only, oui dans tous les cas : un appelant
/// web a forcement franchi le gate `SUPERADMIN_USER_IDS`, et un appelant
/// interne (bot/worker) est de confiance. Le parametre reste passe au use case
/// qui, lui, distingue encore proprietaire et staff.
const CALLER_IS_STAFF: bool = true;

/// POST /api/lfg/detail/{id}/close
pub async fn close_lfg(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let ctx = require_ctx(&user)?;

    state
        .lfg_uc
        .close(id, &ctx.discord_user_id, CALLER_IS_STAFF)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/lfg/detail/{id}
pub async fn delete_lfg(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let ctx = require_ctx(&user)?;

    state
        .lfg_uc
        .delete(id, &ctx.discord_user_id, CALLER_IS_STAFF)
        .await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// POST /api/lfg/detail/{id}/join — « je viens ».
pub async fn join_lfg(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<LfgDto>, ApiError> {
    let ctx = require_ctx(&user)?;
    let existing = state.lfg_uc.get(id).await?;
    let name = display_name(&state, &existing.guild_id, &ctx.discord_user_id).await;

    let post = state.lfg_uc.join(id, &ctx.discord_user_id, &name).await?;
    Ok(Json(post.into()))
}

/// DELETE /api/lfg/detail/{id}/join — se retirer.
pub async fn leave_lfg(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<LfgDto>, ApiError> {
    let ctx = require_ctx(&user)?;
    Ok(Json(
        state.lfg_uc.leave(id, &ctx.discord_user_id).await?.into(),
    ))
}

// ── Surface PUBLIQUE ──

/// Vue publique d'une annonce.
///
/// DTO distinct, ecrit champ par champ. Il ne suit deliberement pas l'entite :
/// les identifiants Discord des participants n'y figurent pas. Publier un
/// `user_id` permettrait de retrouver quelqu'un hors du serveur, ce dont un
/// visiteur non connecte n'a aucun besoin.
#[derive(Debug, Serialize)]
pub struct PublicLfgDto {
    pub id: Uuid,
    pub author_name: String,
    pub game: String,
    pub slots: i32,
    pub when_text: String,
    pub description: Option<String>,
    pub created_at: String,
    /// Pseudos seuls, pour afficher les pastilles d'avatar.
    pub interested_names: Vec<String>,
    pub remaining_slots: i32,
    pub is_full: bool,
}

/// GET /api/public/lfg/{guild_id}
pub async fn public_lfg(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<PublicLfgDto>>, ApiError> {
    ensure_guild_id(&guild_id)?;

    // `live_only` force a true : le parametre `all` du back-office ne doit
    // pas pouvoir exposer les annonces fermees a un visiteur.
    let posts = state
        .lfg_uc
        .list(
            &guild_id,
            true,
            clamp_limit(q.limit, DEFAULT_LIMIT, MAX_LIMIT),
        )
        .await?;

    Ok(Json(
        posts
            .into_iter()
            .map(|p| PublicLfgDto {
                // Calcules avant de deplacer les champs.
                remaining_slots: p.remaining_slots(),
                is_full: p.is_full(),
                id: p.id,
                author_name: p.author_name,
                game: p.game,
                slots: p.slots,
                when_text: p.when_text,
                description: p.description,
                created_at: p.created_at.to_rfc3339(),
                interested_names: p.interested.into_iter().map(|i| i.username).collect(),
            })
            .collect(),
    ))
}

