//! GET /api/ai-dataset/messages — liste paginee des messages utilisateurs
//! pour construction d'un dataset d'entrainement IA.
//! DELETE /api/ai-dataset/messages — suppression en masse des messages exportes.
//!
//! Adaptateur ENTRANT mince : user + parse/map. Le bornage des filtres et la
//! validation des ids vivent dans `ManageDatasetUseCase` ; le SQL dans
//! `DatasetRepository`.
//!
//! Gate :
//!   - GET : admin+ (lecture du contenu de chat)
//!   - DELETE : owner+ (action destructive)

use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::bootstrap::state::AiState;
use platform_core::sentinel::ports::inbound::ai::manage_dataset::{
    BulkDeleteCommand, ListDatasetQuery,
};

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    pub channel_id: Option<String>,
    pub from: Option<String>, // ISO8601
    pub to: Option<String>,
    pub min_length: Option<i32>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct DatasetMessageDto {
    pub id: String,
    pub user_id: String,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListMessagesResponse {
    pub items: Vec<DatasetMessageDto>,
    pub total: i64,
}

/// GET /api/ai-dataset/messages/{guild_id}
pub async fn list_messages(
    State(state): State<AiState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(q): Query<ListMessagesQuery>,
) -> Result<Json<ListMessagesResponse>, ApiError> {
    // Securite DoS : borne le limit. Sans plafond, un limit absent devient
    // `LIMIT NULL` en Postgres (= toute la table de messages) et un grand limit
    // rapatrie un volume enorme -> risque OOM. Defaut 500, max 2000.
    let limit = Some(
        crate::sentinel::adapters::inbound::http::helpers::normalize_in(q.limit, 500, 1, 2000),
    );
    let offset = Some(q.offset.unwrap_or(0).max(0));

    let page = state
        .dataset_uc
        .list_messages(ListDatasetQuery {
            guild_id,
            channel_id: q.channel_id,
            from: q.from,
            to: q.to,
            min_length: q.min_length,
            limit,
            offset,
        })
        .await?;

    let items = page
        .items
        .into_iter()
        .map(|m| DatasetMessageDto {
            id: m.id,
            user_id: m.user_id,
            channel_id: m.channel_id,
            channel_name: m.channel_name,
            content: m.content,
            created_at: m.created_at,
        })
        .collect();

    Ok(Json(ListMessagesResponse {
        items,
        total: page.total,
    }))
}

#[derive(Debug, Deserialize)]
pub struct BulkDeleteDto {
    pub ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BulkDeleteResponse {
    pub deleted: i64,
}

/// DELETE /api/ai-dataset/messages/{guild_id}
pub async fn bulk_delete(
    State(state): State<AiState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(body): Json<BulkDeleteDto>,
) -> Result<Json<BulkDeleteResponse>, ApiError> {
    let deleted = state
        .dataset_uc
        .bulk_delete(BulkDeleteCommand {
            guild_id,
            ids: body.ids,
        })
        .await?;

    Ok(Json(BulkDeleteResponse { deleted }))
}
