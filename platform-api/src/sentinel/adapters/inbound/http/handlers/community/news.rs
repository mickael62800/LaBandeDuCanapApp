//! Annonces du site : redaction par le staff, lecture publique.
//!
//! Distinct de `announcements`, qui pilote des messages Discord recurrents
//! postes par le bot. Melanger les deux ferait remonter « pensez a bump ! »
//! dans les nouvelles du site.

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
use platform_core::sentinel::domain::entities::community::news::{NewsPost, UpsertNewsCommand};
use platform_core::sentinel::domain::errors::DomainError;

const DEFAULT_LIMIT: i64 = 5;
const MAX_LIMIT: i64 = 30;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    /// Back-office : inclure les brouillons et les nouvelles programmees.
    #[serde(
        default,
        deserialize_with = "crate::sentinel::adapters::inbound::http::helpers::bool_souple"
    )]
    pub all: bool,
}

#[derive(Debug, Serialize)]
pub struct NewsDto {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub image_url: Option<String>,
    pub is_pinned: bool,
    pub is_public: bool,
    pub published_at: String,
    pub created_by: String,
}

impl From<NewsPost> for NewsDto {
    fn from(n: NewsPost) -> Self {
        Self {
            id: n.id,
            title: n.title,
            body: n.body,
            image_url: n.image_url,
            is_pinned: n.is_pinned,
            is_public: n.is_public,
            published_at: n.published_at.to_rfc3339(),
            created_by: n.created_by,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertNewsDto {
    pub title: String,
    pub body: String,
    pub image_url: Option<String>,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default = "default_true")]
    pub is_public: bool,
    /// RFC3339. Absente a la creation : maintenant. Absente en modification :
    /// la date existante est conservee.
    pub published_at: Option<String>,
}

fn default_true() -> bool {
    true
}

fn parse_published(s: Option<&str>) -> Result<Option<chrono::DateTime<chrono::Utc>>, ApiError> {
    match s {
        Some(v) => chrono::DateTime::parse_from_rfc3339(v)
            .map(|d| Some(d.with_timezone(&chrono::Utc)))
            .map_err(|_| {
                ApiError(DomainError::ValidationError(
                    "published_at invalide (RFC3339)".into(),
                ))
            }),
        None => Ok(None),
    }
}

// ── Back-office ──

/// GET /api/news/{guild_id}
pub async fn list_news(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<NewsDto>>, ApiError> {
    let items = state
        .news_uc
        .list(
            &guild_id,
            !q.all,
            clamp_limit(q.limit, DEFAULT_LIMIT, MAX_LIMIT),
        )
        .await?;
    Ok(Json(items.into_iter().map(Into::into).collect()))
}

/// POST /api/news/{guild_id}
pub async fn create_news(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Json(dto): Json<UpsertNewsDto>,
) -> Result<Json<NewsDto>, ApiError> {
    let cmd = UpsertNewsCommand {
        guild_id,
        title: dto.title,
        body: dto.body,
        image_url: dto.image_url,
        is_pinned: dto.is_pinned,
        is_public: dto.is_public,
        published_at: parse_published(dto.published_at.as_deref())?,
        created_by: user
            .as_ref()
            .map(|Extension(c)| c.discord_user_id.clone())
            .unwrap_or_default(),
    };
    Ok(Json(state.news_uc.create(cmd).await?.into()))
}

/// PUT /api/news/detail/{id}
pub async fn update_news(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpsertNewsDto>,
) -> Result<Json<NewsDto>, ApiError> {
    let existing = state.news_uc.get(id).await?;

    let cmd = UpsertNewsCommand {
        guild_id: existing.guild_id,
        title: dto.title,
        body: dto.body,
        image_url: dto.image_url,
        is_pinned: dto.is_pinned,
        is_public: dto.is_public,
        published_at: parse_published(dto.published_at.as_deref())?,
        // L'auteur d'origine reste l'auteur.
        created_by: existing.created_by,
    };
    Ok(Json(state.news_uc.update(id, cmd).await?.into()))
}

/// DELETE /api/news/detail/{id}
pub async fn delete_news(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.news_uc.get(id).await?;

    state.news_uc.delete(id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ── Surface PUBLIQUE ──

/// Vue publique. Ni `created_by` — savoir quel moderateur a redige n'apporte
/// rien au visiteur — ni `is_public`, qui n'a de sens que cote back-office.
///
/// `excerpt` accompagne `body` : la liste affiche l'extrait, la fiche le
/// texte complet, sans second aller-retour.
#[derive(Debug, Serialize)]
pub struct PublicNewsDto {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub excerpt: String,
    pub image_url: Option<String>,
    pub is_pinned: bool,
    pub published_at: String,
}

/// GET /api/public/news/{guild_id}
pub async fn public_news(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<PublicNewsDto>>, ApiError> {
    ensure_guild_id(&guild_id)?;

    // `published_only` force a true : `?all=1` ne doit exposer ni les
    // brouillons, ni les nouvelles programmees pour plus tard.
    let items = state
        .news_uc
        .list(
            &guild_id,
            true,
            clamp_limit(q.limit, DEFAULT_LIMIT, MAX_LIMIT),
        )
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(|n| PublicNewsDto {
                // Calcule avant de deplacer `body`.
                excerpt: n.excerpt(),
                id: n.id,
                title: n.title,
                body: n.body,
                image_url: n.image_url,
                is_pinned: n.is_pinned,
                published_at: n.published_at.to_rfc3339(),
            })
            .collect(),
    ))
}
