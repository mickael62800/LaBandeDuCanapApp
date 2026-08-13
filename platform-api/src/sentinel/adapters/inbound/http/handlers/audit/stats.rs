use crate::sentinel::adapters::inbound::http::extractors::{ValidatedGuild, ValidatedGuildUser};
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::sentinel::adapters::inbound::http::dto::audit::stats::GuildOverviewDto;
use crate::sentinel::adapters::inbound::http::dto::audit::stats::GuildVoiceStatsDto;
use crate::sentinel::adapters::inbound::http::dto::audit::stats::LeaderboardQuery;
use crate::sentinel::adapters::inbound::http::dto::audit::stats::RecordMessagesDto;
use crate::sentinel::adapters::inbound::http::dto::audit::stats::RecordVoiceDto;
use crate::sentinel::adapters::inbound::http::dto::audit::stats::UserStatsDto;
use crate::sentinel::adapters::inbound::http::dto::audit::stats::VoiceStatsQuery;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::bootstrap::state::AuditState;

/// POST /api/stats/messages — enregistrer des messages
pub async fn record_messages(
    State(state): State<AuditState>,
    Json(dto): Json<RecordMessagesDto>,
) -> Result<StatusCode, ApiError> {
    let guild_id = dto.guild_id.clone();
    let user_id = dto.user_id.clone();
    let count = dto.count;

    state.stats_uc.record_messages(dto.into()).await?;

    state.broadcaster.broadcast(
        "stats_messages_recorded",
        serde_json::json!({ "guild_id": &guild_id, "user_id": &user_id, "count": count }),
    );

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/stats/voice — enregistrer du temps vocal
pub async fn record_voice(
    State(state): State<AuditState>,
    Json(dto): Json<RecordVoiceDto>,
) -> Result<StatusCode, ApiError> {
    let guild_id = dto.guild_id.clone();
    let user_id = dto.user_id.clone();
    let seconds = dto.seconds;

    state.stats_uc.record_voice(dto.into()).await?;

    state.broadcaster.broadcast(
        "stats_voice_recorded",
        serde_json::json!({ "guild_id": &guild_id, "user_id": &user_id, "seconds": seconds }),
    );

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/stats/{guild_id}/user/{user_id} — stats d'un utilisateur
pub async fn get_user_stats(
    State(state): State<AuditState>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<Option<UserStatsDto>>, ApiError> {
    // IDOR : les GET echappent au gate global -> exiger l'appartenance au serveur.
    let stats = state.stats_uc.get_user_stats(&guild_id, &user_id).await?;
    Ok(Json(stats.map(UserStatsDto::from)))
}

/// GET /api/stats/{guild_id}/overview — stats globales du serveur
pub async fn get_guild_overview(
    State(state): State<AuditState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<GuildOverviewDto>, ApiError> {
    let overview = state.stats_uc.get_guild_overview(&guild_id).await?;
    Ok(Json(GuildOverviewDto::from(overview)))
}

/// GET /api/stats/{guild_id}/leaderboard — classement
pub async fn get_leaderboard(
    State(state): State<AuditState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<LeaderboardQuery>,
) -> Result<Json<Vec<UserStatsDto>>, ApiError> {
    let limit = params.limit.unwrap_or(10).min(50);
    let members = state.stats_uc.get_leaderboard(&guild_id, limit).await?;
    Ok(Json(members.into_iter().map(UserStatsDto::from).collect()))
}

/// GET /api/stats/{guild_id}/voice-stats — stats vocales par salon
pub async fn get_guild_voice_stats(
    State(state): State<AuditState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<VoiceStatsQuery>,
) -> Result<Json<GuildVoiceStatsDto>, ApiError> {
    let days = params.days.unwrap_or(30).min(90);
    let limit = params.limit.unwrap_or(20).min(50);
    let stats = state
        .stats_uc
        .get_guild_voice_stats(&guild_id, days, limit)
        .await?;
    Ok(Json(GuildVoiceStatsDto::from(stats)))
}
