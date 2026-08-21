//! Endpoint GET /api/games/servers/{server_id}/sessions — historique
//! des sessions joueurs (joined_at / left_at / duration_seconds).

use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::nexus::adapters::inbound::http::handlers::ApiError;
use crate::nexus::bootstrap::AppState;
use platform_core::nexus::domain::entities::game::player_session::PlayerSession;

#[derive(Debug, Deserialize)]
pub struct SessionsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PlayerSessionDto {
    pub id: Uuid,
    pub server_id: Uuid,
    pub player_name: String,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
}

impl From<PlayerSession> for PlayerSessionDto {
    fn from(s: PlayerSession) -> Self {
        Self {
            id: s.id,
            server_id: s.server_id,
            player_name: s.player_name,
            joined_at: s.joined_at,
            left_at: s.left_at,
            duration_seconds: s.duration_seconds,
        }
    }
}

/// Une page d'historique, et de quoi savoir ce qu'il y a derriere.
///
/// La reponse etait une simple liste : l'ecran ne pouvait donc afficher ni le
/// nombre de pages, ni le total. Il montrait les cent dernieres sessions en
/// laissant croire que c'etait tout l'historique.
#[derive(Debug, Serialize)]
pub struct PageDeSessions {
    pub items: Vec<PlayerSessionDto>,
    /// Nombre total de sessions du serveur, toutes pages confondues.
    pub total: i64,
}

pub async fn list_sessions(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<SessionsQuery>,
) -> Result<Json<PageDeSessions>, ApiError> {
    let limit = q.limit.unwrap_or(100).clamp(1, 1000);
    let offset = q.offset.unwrap_or(0).max(0);
    let list = state
        .game_session_repo
        .list_history(server_id, limit, offset)
        .await?;
    let total = state.game_session_repo.count_history(server_id).await?;
    Ok(Json(PageDeSessions {
        items: list.into_iter().map(PlayerSessionDto::from).collect(),
        total,
    }))
}
