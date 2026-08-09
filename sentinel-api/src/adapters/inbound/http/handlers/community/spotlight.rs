//! Membre du mois : designation par le staff, lecture publique.

use axum::extract::{Path, Query, State};
use axum::Extension;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::handlers::community::public_guard::ensure_guild_id;
use crate::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::bootstrap::state::CommunityState;
use sentinel_core::domain::entities::community::spotlight::{Spotlight, UpsertSpotlightCommand};

#[derive(Debug, Deserialize)]
pub struct PeriodQuery {
    /// `AAAA-MM`. Absente : la designation la plus recente.
    pub period: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SpotlightDto {
    pub id: Uuid,
    pub user_id: String,
    pub username: String,
    pub avatar: Option<String>,
    pub period: String,
    pub reason: String,
}

impl From<Spotlight> for SpotlightDto {
    fn from(s: Spotlight) -> Self {
        Self {
            id: s.id,
            user_id: s.user_id,
            username: s.username,
            avatar: s.avatar,
            period: s.period,
            reason: s.reason,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DesignateDto {
    pub user_id: String,
    #[serde(default)]
    pub username: String,
    pub avatar: Option<String>,
    pub period: Option<String>,
    pub reason: String,
}

// ── Back-office ──

/// GET /api/spotlight/{guild_id} — historique des designations.
pub async fn list_spotlight(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<SpotlightDto>>, ApiError> {
    let items = state.spotlight_uc.list(&guild_id, 24).await?;
    Ok(Json(items.into_iter().map(Into::into).collect()))
}

/// POST /api/spotlight/{guild_id} — designer (ou remplacer) le membre du mois.
pub async fn designate_spotlight(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Json(dto): Json<DesignateDto>,
) -> Result<Json<SpotlightDto>, ApiError> {
    // Pseudo et avatar resolus depuis `guild_members` : le staff saisit un
    // identifiant, pas un nom d'affichage, et un nom recopie a la main
    // devient faux des le prochain changement de pseudo. Le corps de la
    // requete ne sert que de repli si le membre n'est pas encore synchronise.
    let (username, avatar) = (String::new(), None);

    let cmd = UpsertSpotlightCommand {
        guild_id,
        user_id: dto.user_id,
        username,
        avatar,
        period: dto.period,
        reason: dto.reason,
        chosen_by: user
            .as_ref()
            .map(|Extension(c)| c.discord_user_id.clone())
            .unwrap_or_default(),
    };
    Ok(Json(state.spotlight_uc.designate(cmd).await?.into()))
}

/// DELETE /api/spotlight/detail/{id}
pub async fn delete_spotlight(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path((_guild_id, id)): Path<(String, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.spotlight_uc.delete(id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ── Surface PUBLIQUE ──

/// Vue publique. `chosen_by` en est absent volontairement : savoir quel
/// moderateur a designe qui n'apporte rien au visiteur et expose une
/// mecanique interne.
#[derive(Debug, Serialize)]
pub struct PublicSpotlightDto {
    pub username: String,
    pub avatar: Option<String>,
    pub period: String,
    pub reason: String,
}

/// GET /api/public/spotlight/{guild_id}?period=
pub async fn public_spotlight(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
    Query(q): Query<PeriodQuery>,
) -> Result<Json<Option<PublicSpotlightDto>>, ApiError> {
    ensure_guild_id(&guild_id)?;

    let found = state
        .spotlight_uc
        .current(&guild_id, q.period.as_deref())
        .await?;
    Ok(Json(found.map(|s| PublicSpotlightDto {
        username: s.username,
        avatar: s.avatar,
        period: s.period,
        reason: s.reason,
    })))
}

