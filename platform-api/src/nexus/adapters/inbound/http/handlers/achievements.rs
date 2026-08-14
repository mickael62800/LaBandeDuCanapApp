//! Handlers HTTP des hauts faits.
//!
//! Deux publics :
//!   - le DASHBOARD, qui lit le catalogue et choisit l'image de chaque haut
//!     fait (`PATCH /definitions/{id}`) ;
//!   - le BOT, qui lie l'identite de jeu d'un membre, lit sa progression et
//!     relaie les evenements de jeu.
//!
//! Toute attribution reussie publie `achievement.unlocked` sur `nexus:events`.
//! La publication a lieu APRES confirmation de la persistance : un echec
//! Discord ne doit jamais annuler un haut fait deja enregistre.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::nexus::adapters::inbound::http::handlers::{validate_discord_id, ApiError};
use crate::nexus::bootstrap::AppState;
use platform_core::nexus::domain::entities::achievement::{Achievement, AchievementProgress};
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::inbound::achievements::{GameUnlockCommand, UnlockOutcome};
use platform_core::nexus::ports::outbound::achievement_repository::AchievementUpdate;
use platform_core::nexus::ports::outbound::events::achievement_events::ACHIEVEMENT_UNLOCKED;

// ── DTO ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AchievementDto {
    pub id: Uuid,
    pub game: Option<String>,
    pub code: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub icon_url: Option<String>,
    pub criteria: serde_json::Value,
    pub verification: String,
    pub hidden: bool,
    pub enabled: bool,
}

impl From<Achievement> for AchievementDto {
    fn from(a: Achievement) -> Self {
        Self {
            id: a.id,
            game: a.game,
            code: a.code,
            name: a.name,
            description: a.description,
            category: a.category,
            icon_url: a.icon_url,
            criteria: a.criteria,
            verification: a.verification.as_str().to_owned(),
            hidden: a.hidden,
            enabled: a.enabled,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AchievementProgressDto {
    #[serde(flatten)]
    pub achievement: AchievementDto,
    pub unlocked_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<AchievementProgress> for AchievementProgressDto {
    fn from(p: AchievementProgress) -> Self {
        Self {
            achievement: p.achievement.into(),
            unlocked_at: p.unlocked_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GameQuery {
    pub game: Option<String>,
}

/// Mise a jour d'une definition depuis le dashboard.
///
/// `icon_url` distingue trois cas : absent = ne pas toucher, `null` = effacer
/// l'image, chaine = nouvelle image. C'est ce qui permet a l'admin de retirer
/// une image sans devoir en fournir une autre.
#[derive(Debug, Deserialize)]
pub struct UpdateAchievementDto {
    #[serde(default, deserialize_with = "double_option")]
    pub icon_url: Option<Option<String>>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub hidden: Option<bool>,
    pub criteria: Option<serde_json::Value>,
}

fn double_option<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize)]
pub struct LinkIdentityDto {
    /// Identite dans le jeu. Palworld : SteamID64 (17 chiffres).
    pub game_player_id: String,
}

#[derive(Debug, Serialize)]
pub struct LinkDto {
    pub game: String,
    pub game_player_id: String,
    pub verified: bool,
    pub verified_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct GrantDto {
    pub discord_user_id: String,
    pub achievement_id: Uuid,
    /// Acteur pour les appelants internes (le bot). Ignore pour la passerelle.
    pub actor_id: Option<String>,
}

/// Evenement normalise produit par un adaptateur de jeu.
#[derive(Debug, Deserialize)]
pub struct GameEventDto {
    pub game: String,
    pub game_player_id: String,
    pub achievement_code: String,
    pub source_event_id: String,
}

#[derive(Debug, Serialize)]
pub struct UnlockResultDto {
    /// `false` quand le membre le possedait deja ou que l'evenement avait deja
    /// ete consomme : l'appelant sait qu'il ne doit rien annoncer.
    pub unlocked: bool,
}

// ── Acteur ───────────────────────────────────────────────────────────────

const EN_TETE_SOURCE: &str = "x-actor-source";
const EN_TETE_ACTEUR: &str = "x-actor-id";

/// Meme regime que les handlers game : la passerelle impose l'identite qu'elle
/// a verifiee et le parametre du corps est alors IGNORE ; un appelant interne
/// (le bot, porteur de la cle API) peut nommer l'acteur.
fn acteur(headers: &HeaderMap, depuis_corps: Option<&str>) -> String {
    let non_vide = |v: &str| {
        let v = v.trim();
        (!v.is_empty()).then(|| v.to_owned())
    };
    if headers.contains_key(EN_TETE_SOURCE) {
        return headers
            .get(EN_TETE_ACTEUR)
            .and_then(|v| v.to_str().ok())
            .and_then(non_vide)
            .unwrap_or_else(|| "inconnu".to_owned());
    }
    depuis_corps
        .and_then(non_vide)
        .unwrap_or_else(|| "inconnu".to_owned())
}

// ── Catalogue (dashboard) ────────────────────────────────────────────────

/// GET /api/achievements/definitions?game=palworld
pub async fn list_definitions(
    State(state): State<AppState>,
    Query(q): Query<GameQuery>,
) -> Result<Json<Vec<AchievementDto>>, ApiError> {
    let definitions = state
        .achievements_uc
        .list_definitions(q.game.as_deref())
        .await?;
    Ok(Json(definitions.into_iter().map(Into::into).collect()))
}

/// PATCH /api/achievements/definitions/{id}
///
/// Point d'entree du choix d'image par l'administrateur.
pub async fn update_definition(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateAchievementDto>,
) -> Result<Json<AchievementDto>, ApiError> {
    let update = AchievementUpdate {
        icon_url: dto.icon_url,
        name: dto.name,
        description: dto.description,
        enabled: dto.enabled,
        hidden: dto.hidden,
        criteria: dto.criteria,
    };
    let updated = state.achievements_uc.update_definition(id, update).await?;
    Ok(Json(updated.into()))
}

// ── Consultation ─────────────────────────────────────────────────────────

/// GET /api/achievements/{guild_id}/members/{user_id}?game=palworld
pub async fn member_progress(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Query(q): Query<GameQuery>,
) -> Result<Json<Vec<AchievementProgressDto>>, ApiError> {
    validate_discord_id("guild_id", &guild_id)?;
    validate_discord_id("user_id", &user_id)?;
    let progress = state
        .achievements_uc
        .member_progress(&guild_id, &user_id, q.game.as_deref())
        .await?;
    Ok(Json(progress.into_iter().map(Into::into).collect()))
}

// ── Liaison d'identite de jeu ────────────────────────────────────────────

/// GET /api/achievements/{guild_id}/links/{user_id}/{game}
pub async fn get_link(
    State(state): State<AppState>,
    Path((guild_id, user_id, game)): Path<(String, String, String)>,
) -> Result<Json<Option<LinkDto>>, ApiError> {
    validate_discord_id("guild_id", &guild_id)?;
    validate_discord_id("user_id", &user_id)?;
    let link = state
        .achievements_uc
        .find_link(&guild_id, &user_id, &game)
        .await?;
    Ok(Json(link.map(|l| LinkDto {
        game: l.game,
        game_player_id: l.game_player_id,
        verified: l.verified_at.is_some(),
        verified_at: l.verified_at,
    })))
}

/// PUT /api/achievements/{guild_id}/links/{user_id}/{game}
pub async fn put_link(
    State(state): State<AppState>,
    Path((guild_id, user_id, game)): Path<(String, String, String)>,
    Json(dto): Json<LinkIdentityDto>,
) -> Result<Json<LinkDto>, ApiError> {
    validate_discord_id("guild_id", &guild_id)?;
    validate_discord_id("user_id", &user_id)?;
    let link = state
        .achievements_uc
        .link_identity(&guild_id, &user_id, &game, &dto.game_player_id)
        .await?;
    Ok(Json(LinkDto {
        game: link.game,
        game_player_id: link.game_player_id,
        verified: link.verified_at.is_some(),
        verified_at: link.verified_at,
    }))
}

/// DELETE /api/achievements/{guild_id}/links/{user_id}/{game}
pub async fn delete_link(
    State(state): State<AppState>,
    Path((guild_id, user_id, game)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    validate_discord_id("guild_id", &guild_id)?;
    validate_discord_id("user_id", &user_id)?;
    let removed = state
        .achievements_uc
        .unlink_identity(&guild_id, &user_id, &game)
        .await?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(DomainError::NotFound("aucune identite liee".into()).into())
    }
}

// ── Attribution ──────────────────────────────────────────────────────────

/// Publie l'annonce sur `nexus:events`, apres persistance confirmee.
async fn publish_unlock(state: &AppState, outcome: &UnlockOutcome) -> bool {
    let UnlockOutcome::Unlocked(unlocked) = outcome else {
        return false;
    };
    state
        .events
        .publish(
            ACHIEVEMENT_UNLOCKED,
            serde_json::json!({
                "guild_id": unlocked.guild_id,
                "discord_user_id": unlocked.discord_user_id,
                "achievement_id": unlocked.achievement.id.to_string(),
                "achievement_code": unlocked.achievement.code,
                "achievement_name": unlocked.achievement.name,
                "achievement_description": unlocked.achievement.description,
                "icon_url": unlocked.achievement.icon_url,
                "game": unlocked.achievement.game,
                "source_event_id": unlocked.source_event_id,
            }),
        )
        .await;
    true
}

/// POST /api/achievements/{guild_id}/grant — attribution manuelle (admin).
pub async fn grant(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    headers: HeaderMap,
    Json(dto): Json<GrantDto>,
) -> Result<Json<UnlockResultDto>, ApiError> {
    validate_discord_id("guild_id", &guild_id)?;
    validate_discord_id("discord_user_id", &dto.discord_user_id)?;
    let actor = acteur(&headers, dto.actor_id.as_deref());

    let outcome = state
        .achievements_uc
        .grant_manually(&guild_id, &dto.discord_user_id, dto.achievement_id, &actor)
        .await?;
    let unlocked = publish_unlock(&state, &outcome).await;
    Ok(Json(UnlockResultDto { unlocked }))
}

/// POST /api/achievements/{guild_id}/game-events
///
/// Relais des evenements produits par un adaptateur de jeu. Le membre Discord
/// n'est jamais fourni par l'appelant : il est resolu par la liaison verifiee.
pub async fn game_event(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<GameEventDto>,
) -> Result<Json<UnlockResultDto>, ApiError> {
    validate_discord_id("guild_id", &guild_id)?;
    let outcome = state
        .achievements_uc
        .unlock_from_game_event(GameUnlockCommand {
            guild_id,
            game: dto.game,
            game_player_id: dto.game_player_id,
            achievement_code: dto.achievement_code,
            source_event_id: dto.source_event_id,
        })
        .await?;
    let unlocked = publish_unlock(&state, &outcome).await;
    Ok(Json(UnlockResultDto { unlocked }))
}
