//! Catalogue des jeux mentionnables (games) + panels Discord — version nexus.
//!
//! Differences avec sentinel-api :
//! - Pas de RBAC (Bearer global uniquement).
//! - Pas d'adapter Discord API cote nexus-api : les roles du workflow Web
//!   sont crees/supprimes par `nexus-bot` via le stream d'evenements. Le
//!   workflow slash peut toujours fournir directement son `role_id`.
//! - Les operations Discord asynchrones, comme `panel/deploy`, sont publiees
//!   sur le stream Nexus puis executees par `nexus-bot`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::handlers::ApiError;
use crate::bootstrap::AppState;
use nexus_core::domain::entities::casino::game::{normalize_game_name, normalize_optional_tag};
use nexus_core::domain::entities::system::discord_ids::{ChannelId, GuildId, MessageId};
use nexus_core::domain::errors::DomainError;
use nexus_core::ports::outbound::casino::game_repository::{Game, GamePanel};

// ── DTOs ──

#[derive(Debug, Serialize)]
pub struct GameDto {
    pub id: String,
    pub guild_id: GuildId,
    pub game_name: String,
    pub created_by: String,
    pub created_at: String,
    pub emoji: Option<String>,
    pub category: Option<String>,
    pub role_id: Option<String>,
}

impl From<Game> for GameDto {
    fn from(g: Game) -> Self {
        Self {
            id: g.id,
            guild_id: g.guild_id,
            game_name: g.game_name,
            created_by: g.created_by,
            created_at: g.created_at,
            emoji: g.emoji,
            category: g.category,
            role_id: g.role_id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateGameDto {
    pub guild_id: GuildId,
    pub game_name: String,
    pub created_by: String,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    /// Role Discord associe au jeu, cree par le bot avant l'appel.
    #[serde(default)]
    pub role_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetRoleIdDto {
    // `null` = reset a NULL, absent = NOT_PROVIDED ; ici on traite both comme "null ou valeur".
    #[serde(default)]
    pub role_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GamePanelDto {
    pub id: String,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub category: Option<String>,
}

impl From<GamePanel> for GamePanelDto {
    fn from(p: GamePanel) -> Self {
        Self {
            id: p.id,
            guild_id: p.guild_id,
            channel_id: p.channel_id,
            message_id: p.message_id,
            category: p.category,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SavePanelDto {
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    #[serde(default)]
    pub category: Option<String>,
}

// ── Games CRUD (via GameRepository) ──

pub async fn list_games(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<GameDto>>, ApiError> {
    let games = state.game_repo.list(&guild_id).await?;
    Ok(Json(games.into_iter().map(Into::into).collect()))
}

pub async fn create_game(
    State(state): State<AppState>,
    Json(dto): Json<CreateGameDto>,
) -> Result<Json<GameDto>, ApiError> {
    let name = normalize_game_name(&dto.game_name)
        .map_err(|m| ApiError(DomainError::ValidationError(m.into())))?;
    let emoji_owned = normalize_optional_tag(dto.emoji.as_deref());
    let category_owned = normalize_optional_tag(dto.category.as_deref());
    let role_owned = normalize_optional_tag(dto.role_id.as_deref());

    let game = state
        .game_repo
        .create(
            &dto.guild_id,
            &name,
            &dto.created_by,
            emoji_owned.as_deref(),
            category_owned.as_deref(),
            role_owned.as_deref(),
        )
        .await?;

    // Le Web ne peut pas creer un role Discord directement. Demande au bot
    // de provisionner les roles manquants, sans attendre le deploiement d'un
    // panneau. Le workflow slash fournit deja son role_id et ne republie pas.
    if game.role_id.is_none() {
        use nexus_core::ports::outbound::events::game_events;
        state
            .events
            .publish(
                game_events::GAMES_ROLES_ENSURE,
                json!({ "guild_id": dto.guild_id.as_ref() }),
            )
            .await;
    }
    Ok(Json(game.into()))
}

#[derive(Debug, Deserialize)]
pub struct UpdateGameDto {
    #[serde(default)]
    pub game_name: Option<String>,
    // emoji/category : `null` = mettre a NULL, absent = ne pas toucher.
    #[serde(default, deserialize_with = "deserialize_opt_opt")]
    pub emoji: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_opt_opt")]
    pub category: Option<Option<String>>,
}

fn deserialize_opt_opt<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = Option::<Option<String>>::deserialize(deserializer)?;
    Ok(Some(v.unwrap_or(None)))
}

pub async fn update_game(
    State(state): State<AppState>,
    Path((guild_id, game_id)): Path<(String, String)>,
    Json(dto): Json<UpdateGameDto>,
) -> Result<Json<GameDto>, ApiError> {
    let name_owned: Option<String> = match dto.game_name.as_deref() {
        Some(raw) if !raw.trim().is_empty() => Some(
            normalize_game_name(raw)
                .map_err(|m| ApiError(DomainError::ValidationError(m.into())))?,
        ),
        _ => None,
    };

    let emoji: Option<Option<String>> = dto
        .emoji
        .as_ref()
        .map(|opt| normalize_optional_tag(opt.as_deref()));
    let category: Option<Option<String>> = dto
        .category
        .as_ref()
        .map(|opt| normalize_optional_tag(opt.as_deref()));

    let updated = state
        .game_repo
        .update(
            &guild_id,
            &game_id,
            name_owned.as_deref(),
            emoji.as_ref().map(|o| o.as_deref()),
            category.as_ref().map(|o| o.as_deref()),
        )
        .await?;
    match updated {
        Some(g) => Ok(Json(g.into())),
        None => Err(DomainError::NotFound("Jeu introuvable".into()).into()),
    }
}

pub async fn delete_game(
    State(state): State<AppState>,
    Path((guild_id, game_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let game = state
        .game_repo
        .list(&guild_id)
        .await?
        .into_iter()
        .find(|game| game.id == game_id)
        .ok_or_else(|| ApiError(DomainError::NotFound("Jeu introuvable".into())))?;

    if !state.game_repo.delete(&guild_id, &game_id).await? {
        return Err(DomainError::NotFound("Jeu introuvable".into()).into());
    }

    if let Some(role_id) = game.role_id {
        use nexus_core::ports::outbound::events::game_events;
        state
            .events
            .publish(
                game_events::GAME_ROLE_DELETE,
                json!({
                    "guild_id": guild_id,
                    "role_id": role_id,
                    "game_name": game.game_name,
                }),
            )
            .await;
    }
    Ok(StatusCode::NO_CONTENT)
}

// ── Role binding ──

/// PATCH /api/games/{guild_id}/{game_id}/role
/// Body: `{ "role_id": "..." | null }` — `null` efface la liaison.
pub async fn set_role_id(
    State(state): State<AppState>,
    Path((guild_id, game_id)): Path<(String, String)>,
    Json(dto): Json<SetRoleIdDto>,
) -> Result<Json<GameDto>, ApiError> {
    let role_owned = normalize_optional_tag(dto.role_id.as_deref());
    let updated = state
        .game_repo
        .set_role_id(&guild_id, &game_id, role_owned.as_deref())
        .await?;
    match updated {
        Some(g) => Ok(Json(g.into())),
        None => Err(DomainError::NotFound("Jeu introuvable".into()).into()),
    }
}

pub async fn get_game_by_name(
    State(state): State<AppState>,
    Path((guild_id, game_name)): Path<(String, String)>,
) -> Result<Json<Option<GameDto>>, ApiError> {
    let game = state.game_repo.find_by_name(&guild_id, &game_name).await?;
    Ok(Json(game.map(Into::into)))
}

// ── Panels ──

pub async fn save_panel(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<SavePanelDto>,
) -> Result<Json<GamePanelDto>, ApiError> {
    let category_owned = normalize_optional_tag(dto.category.as_deref());
    let panel = state
        .game_repo
        .save_panel(
            &guild_id,
            &dto.channel_id,
            &dto.message_id,
            category_owned.as_deref(),
        )
        .await?;
    Ok(Json(panel.into()))
}

#[derive(Debug, Deserialize)]
pub struct SetGameRoleDto {
    /// `null`/absent = dissocier le rôle du jeu.
    pub role_id: Option<String>,
}

/// PUT /api/games/{guild_id}/{game_id}/role — (dé)associe un rôle Discord à un jeu.
///
/// Sert au backfill : le bot crée un rôle pour les jeux legacy sans `role_id`
/// (créés avant le support des rôles) et persiste l'association via cet endpoint.
pub async fn set_game_role(
    State(state): State<AppState>,
    Path((guild_id, game_id)): Path<(String, String)>,
    Json(dto): Json<SetGameRoleDto>,
) -> Result<Json<GameDto>, ApiError> {
    let role = normalize_optional_tag(dto.role_id.as_deref());
    let game = state
        .game_repo
        .set_role_id(&guild_id, &game_id, role.as_deref())
        .await?
        .ok_or_else(|| ApiError(DomainError::ValidationError("jeu introuvable".into())))?;
    Ok(Json(game.into()))
}

pub async fn find_panel_by_message(
    State(state): State<AppState>,
    Path((guild_id, message_id)): Path<(String, String)>,
) -> Result<Json<Option<GamePanelDto>>, ApiError> {
    let panel = state
        .game_repo
        .find_panel_by_message(&guild_id, &message_id)
        .await?;
    Ok(Json(panel.map(Into::into)))
}

pub async fn list_panels(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<GamePanelDto>>, ApiError> {
    let panels = state.game_repo.list_panels(&guild_id).await?;
    Ok(Json(panels.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Deserialize)]
pub struct DeployPanelDto {
    pub channel_id: String,
    pub category: Option<String>,
}

pub async fn deploy_panel(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<DeployPanelDto>,
) -> Result<StatusCode, ApiError> {
    use nexus_core::application::deploy_panel_service::DeployGamesPanelUseCase;
    let uc = DeployGamesPanelUseCase::new(state.events.clone());
    uc.execute(&guild_id, &dto.channel_id, dto.category.as_deref())
        .await;
    Ok(StatusCode::ACCEPTED)
}

pub async fn list_games_by_category(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(q): Query<CategoryQuery>,
) -> Result<Json<Vec<GameDto>>, ApiError> {
    // `category` absente signifie "tous les jeux". Une valeur cible une
    // categorie precise sans tenir compte de la casse.
    let cat_owned = normalize_optional_tag(q.category.as_deref());
    let games = state
        .game_repo
        .list_by_category(&guild_id, cat_owned.as_deref())
        .await?;
    Ok(Json(games.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Deserialize)]
pub struct CategoryQuery {
    #[serde(default)]
    pub category: Option<String>,
}

use axum::extract::Multipart;
use serde_json::json;

pub async fn upload_emoji(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    use nexus_core::application::upload_emoji_service::UploadEmojiUseCase;

    let mut name = String::new();
    let mut image_bytes = Vec::new();
    let mut mime_type = String::from("image/png");

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "name" {
            name = field.text().await.unwrap_or_default();
        } else if field_name == "image" {
            if let Some(content_type) = field.content_type() {
                mime_type = content_type.to_string();
            }
            image_bytes = field.bytes().await.unwrap_or_default().to_vec();
        }
    }

    if name.is_empty() || image_bytes.is_empty() {
        return Err(DomainError::ValidationError("Le nom et l'image sont requis.".into()).into());
    }

    let uc = UploadEmojiUseCase::new(state.discord_api.clone());
    let (id, emoji_name) = uc
        .execute(&guild_id, &name, &image_bytes, &mime_type)
        .await?;

    Ok(Json(json!({ "id": id, "name": emoji_name })))
}

#[derive(Debug, Deserialize)]
pub struct DetectGameMentionsDto {
    pub content: String,
}

pub async fn detect_mentions(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<DetectGameMentionsDto>,
) -> Result<Json<Vec<GameDto>>, ApiError> {
    use nexus_core::application::game_mentions_service::DetectGameMentionsUseCase;
    let uc = DetectGameMentionsUseCase::new(state.game_repo.clone());
    let detected = uc.execute(&guild_id, &dto.content).await?;
    Ok(Json(detected.into_iter().map(Into::into).collect()))
}
