//! Handlers HTTP Game Portal — version nexus.
//!
//! Difference avec sentinel-api : pas de RBAC/component-gates ici, la seule
//! auth est le Bearer global NEXUS_API_KEY (middleware require_api_key).
//! L'identite de l'acteur (audit) vient du payload/query (`actor_id`),
//! comme pour les handlers wallet.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::adapters::inbound::http::dto::game::servers::{
    CreateGameServerDto, GameServerDetailDto, GameServerDto, GameServerStatsDto, RconCommandDto,
    RconCommandResponseDto, UpdateConfigDto,
};
use crate::adapters::inbound::http::handlers::ApiError;
use crate::bootstrap::AppState;
use nexus_core::domain::entities::game::server::CreateGameServerCommand;
use nexus_core::ports::outbound::events::game_events::{
    IP_REVEAL, SERVER_DELETED, SERVER_SCHEDULED, SERVER_STARTED, SERVER_STOPPED,
};

/// POST /api/games/{guild_id}/servers
pub async fn create_server(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<CreateGameServerDto>,
) -> Result<(StatusCode, Json<GameServerDto>), ApiError> {
    let cmd = CreateGameServerCommand {
        guild_id: guild_id.clone(),
        template_slug: dto.template_slug,
        name: dto.name,
        allocated_memory_mb: dto.memory_mb,
        cpu_limit: dto.cpu_limit,
        owner_user_id: dto.owner_user_id,
        initial_config: dto.config,
    };
    let server = state.game_servers_uc.create(cmd).await?;

    // Programme la revelation d'IP : delai fourni, sinon defaut de la guild.
    // 0 jour = pas de revelation programmee.
    let default_days = nexus_core::domain::entities::system::bot_config::cfg_i64(
        &state
            .bot_config_repo
            .get_config(&guild_id, super::GAME_PORTAL_BOT)
            .await
            .unwrap_or_default(),
        "ip_reveal_default_days",
        7,
    ) as i32;
    let days = dto.ip_reveal_days.unwrap_or(default_days).max(0);
    if days > 0 {
        let at = chrono::Utc::now() + chrono::Duration::days(i64::from(days));
        let _ = state
            .game_server_repo
            .set_ip_reveal_at(server.id, Some(at))
            .await;
    }

    let hote = hote_public(&state, &guild_id).await;
    Ok((
        StatusCode::CREATED,
        Json(GameServerDto::from(server).avec_hote(hote.as_deref())),
    ))
}

/// Hote public annonce aux joueurs, lu dans la config game-portal de la guild.
///
/// Le bot le lit deja sous la meme cle pour composer l'adresse au moment de la
/// revelation. On le sert aussi a l'administration, mais sans attendre cette
/// revelation : elle protege l'adresse des JOUEURS, pas des administrateurs,
/// qui ont besoin de tester la connexion avant d'ouvrir la session.
pub(super) async fn hote_public(state: &AppState, guild_id: &str) -> Option<String> {
    let cfg = state
        .bot_config_repo
        .get_config(guild_id, super::GAME_PORTAL_BOT)
        .await
        .unwrap_or_default();
    nexus_core::domain::entities::system::bot_config::cfg_str(&cfg, "session_public_host")
        .filter(|h| !h.trim().is_empty())
        .map(str::to_string)
}

/// GET /api/games/{guild_id}/servers
pub async fn list_servers(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<GameServerDto>>, ApiError> {
    let list = state.game_servers_uc.list_for_guild(&guild_id).await?;
    // Un seul appel de config pour toute la liste : l'hote est commun a la guild.
    let hote = hote_public(&state, &guild_id).await;
    Ok(Json(
        list.into_iter()
            .map(|s| GameServerDto::from(s).avec_hote(hote.as_deref()))
            .collect(),
    ))
}

/// GET /api/games/servers/{server_id}
pub async fn get_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<GameServerDetailDto>, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let hote = hote_public(&state, &detail.server.guild_id).await;
    Ok(Json(
        GameServerDetailDto::from(detail).avec_hote(hote.as_deref()),
    ))
}

#[derive(Debug, Deserialize)]
pub struct ActorQuery {
    /// Discord user id de l'acteur (audit). Si absent, fallback sur l'owner.
    pub actor_id: Option<String>,
}

/// Resout l'acteur pour l'audit : actor_id explicite sinon owner du serveur.
async fn resolve_actor(
    state: &AppState,
    server_id: Uuid,
    explicit: Option<&str>,
) -> Result<String, ApiError> {
    if let Some(s) = explicit {
        return Ok(s.to_string());
    }
    let detail = state.game_servers_uc.get(server_id).await?;
    Ok(detail.server.owner_user_id)
}

/// Publie un evenement de cycle de vie serveur a destination du bot.
/// `guild_id` est lu avant l'action pour rester disponible apres un delete.
async fn publish_lifecycle(state: &AppState, event: &str, server_id: Uuid, guild_id: &str) {
    state
        .events
        .publish(
            event,
            serde_json::json!({
                "server_id": server_id.to_string(),
                "guild_id": guild_id,
            }),
        )
        .await;
}

/// POST /api/games/servers/{server_id}/start
pub async fn start_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = q
        .actor_id
        .clone()
        .unwrap_or_else(|| detail.server.owner_user_id.clone());
    state.game_servers_uc.start(server_id, &actor).await?;
    publish_lifecycle(&state, SERVER_STARTED, server_id, &detail.server.guild_id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/stop
pub async fn stop_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = q
        .actor_id
        .clone()
        .unwrap_or_else(|| detail.server.owner_user_id.clone());
    state.game_servers_uc.stop(server_id, &actor).await?;
    publish_lifecycle(&state, SERVER_STOPPED, server_id, &detail.server.guild_id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/restart
pub async fn restart_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let actor = resolve_actor(&state, server_id, q.actor_id.as_deref()).await?;
    state.game_servers_uc.restart(server_id, &actor).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/reveal-ip
///
/// Force la revelation avant `ip_reveal_at`. La passerelle Web reserve cette
/// route aux administrateurs Nexus ; l'acteur reste journalise dans l'audit.
pub async fn reveal_ip(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = q
        .actor_id
        .clone()
        .unwrap_or_else(|| detail.server.owner_user_id.clone());
    state.game_servers_uc.reveal_ip(server_id, &actor).await?;
    publish_lifecycle(&state, IP_REVEAL, server_id, &detail.server.guild_id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Corps des routes de programmation. `reveal_at` optionnel pour
/// `/reveal-schedule` (None efface la programmation) ; requis pour `/schedule`
/// (une valeur nulle y est refusee par le use case).
#[derive(Debug, Deserialize)]
pub struct ScheduleDto {
    pub reveal_at: Option<chrono::DateTime<chrono::Utc>>,
    pub actor_id: Option<String>,
}

/// POST /api/games/servers/{server_id}/schedule
///
/// Mode « Préparation » : programme l'ouverture sans démarrer le conteneur. Le
/// serveur passe `scheduled` et le bot crée dès maintenant les salons + le
/// panneau d'inscription (événement `game_server_scheduled`). Le worker
/// démarrera le conteneur ~5 min avant l'heure.
pub async fn schedule_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(dto): Json<ScheduleDto>,
) -> Result<StatusCode, ApiError> {
    let reveal_at = dto.reveal_at.ok_or_else(|| {
        ApiError::from(nexus_core::domain::errors::DomainError::ValidationError(
            "reveal_at requis pour programmer l'ouverture".into(),
        ))
    })?;
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = dto
        .actor_id
        .clone()
        .unwrap_or_else(|| detail.server.owner_user_id.clone());
    state
        .game_servers_uc
        .schedule(server_id, reveal_at, &actor)
        .await?;
    publish_lifecycle(&state, SERVER_SCHEDULED, server_id, &detail.server.guild_id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/reveal-schedule
///
/// Programme (ou efface avec `reveal_at` nul) l'heure de révélation auto de
/// l'IP sans changer l'état du conteneur. Complète « Lancer maintenant » quand
/// on veut aussi une révélation automatique.
pub async fn set_reveal_schedule(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(dto): Json<ScheduleDto>,
) -> Result<StatusCode, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = dto
        .actor_id
        .clone()
        .unwrap_or_else(|| detail.server.owner_user_id.clone());
    state
        .game_servers_uc
        .set_reveal_schedule(server_id, dto.reveal_at, &actor)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/games/servers/{server_id}
pub async fn delete_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = q
        .actor_id
        .clone()
        .unwrap_or_else(|| detail.server.owner_user_id.clone());
    state.game_servers_uc.delete(server_id, &actor).await?;
    publish_lifecycle(&state, SERVER_DELETED, server_id, &detail.server.guild_id).await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub lines: Option<u32>,
}

/// GET /api/games/servers/{server_id}/logs?lines=200
pub async fn get_logs(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<Vec<String>>, ApiError> {
    let lines = q.lines.unwrap_or(200).min(1000);
    let logs = state.game_servers_uc.get_logs(server_id, lines).await?;
    Ok(Json(logs))
}

/// GET /api/games/servers/{server_id}/stats
pub async fn get_stats(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<GameServerStatsDto>, ApiError> {
    let stats = state.game_servers_uc.get_stats(server_id).await?;
    Ok(Json(stats.into()))
}

/// PUT /api/games/servers/{server_id}/config
pub async fn update_config(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
    Json(dto): Json<UpdateConfigDto>,
) -> Result<StatusCode, ApiError> {
    let actor = resolve_actor(&state, server_id, q.actor_id.as_deref()).await?;
    state
        .game_servers_uc
        .update_config(server_id, dto.config, &actor)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/command
pub async fn execute_rcon(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ActorQuery>,
    Json(dto): Json<RconCommandDto>,
) -> Result<Json<RconCommandResponseDto>, ApiError> {
    let actor = resolve_actor(&state, server_id, q.actor_id.as_deref()).await?;
    let resp = state
        .game_servers_uc
        .execute_rcon(server_id, &dto.command, &actor)
        .await?;
    Ok(Json(RconCommandResponseDto { response: resp }))
}

use axum::response::sse::{Event, Sse};
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::Stream;

/// GET /api/games/servers/{server_id}/stream-logs?lines=50
pub async fn stream_logs_sse(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<LogsQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let lines = q.lines.unwrap_or(50).min(500);
    let logs = state.game_servers_uc.get_logs(server_id, lines).await?;

    let stream = async_stream::stream! {
        for line in logs {
            yield Ok(Event::default().data(line));
        }
    };

    Ok(Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15))))
}

/// GET /api/games/servers/{server_id}/stream-stats
pub async fn stream_stats_sse(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let stats = state.game_servers_uc.get_stats(server_id).await?;

    let stream = async_stream::stream! {
        if let Ok(json) = serde_json::to_string(&GameServerStatsDto::from(stats)) {
            yield Ok(Event::default().data(json));
        }
    };

    Ok(Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15))))
}
