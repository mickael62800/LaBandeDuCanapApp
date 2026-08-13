//! Handlers HTTP du wallet partage :
//! - GET  /api/wallet/{guild_id}/{user_id}
//! - POST /api/wallet/{guild_id}/transfer
//! - GET  /api/wallet/{guild_id}/{user_id}/history?limit=&offset=
//! - GET  /api/wallet/{guild_id}/leaderboard?limit=

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use platform_core::nexus::domain::entities::wallet::Wallet;
use platform_core::nexus::ports::inbound::transfer_coins::TransferCoinsCommand;
use serde::Deserialize;
use serde::Serialize;

use super::ApiError;
use crate::nexus::bootstrap::AppState;

#[derive(Debug, Serialize)]
pub struct WalletDto {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub coins: i64,
    pub total_earned: i64,
    pub total_spent: i64,
}

impl From<Wallet> for WalletDto {
    fn from(w: Wallet) -> Self {
        Self {
            guild_id: w.guild_id,
            user_id: w.user_id,
            username: w.username,
            coins: w.coins,
            total_earned: w.total_earned,
            total_spent: w.total_spent,
        }
    }
}

pub async fn get(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<WalletDto>, ApiError> {
    let w = state.get_wallet.get(&guild_id, &user_id).await?;
    Ok(Json(w.into()))
}

// ── Transfert ──

#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    pub from_user_id: String,
    #[serde(default)]
    pub from_username: String,
    pub to_user_id: String,
    #[serde(default)]
    pub to_username: String,
    pub amount: i64,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TransferResponse {
    pub amount: i64,
    pub from_balance: i64,
    pub to_balance: i64,
}

pub async fn transfer(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(req): Json<TransferRequest>,
) -> Result<Json<TransferResponse>, ApiError> {
    let res = state
        .transfer_coins
        .transfer(TransferCoinsCommand {
            guild_id,
            from_user_id: req.from_user_id,
            from_username: req.from_username,
            to_user_id: req.to_user_id,
            to_username: req.to_username,
            amount: req.amount,
            reason: req.reason,
        })
        .await?;
    Ok(Json(TransferResponse {
        amount: res.amount,
        from_balance: res.from_balance,
        to_balance: res.to_balance,
    }))
}

// ── Historique ──

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TransactionDto {
    pub id: String,
    pub amount: i64,
    pub balance_after: i64,
    pub source: String,
    pub description: String,
    pub reason: Option<String>,
    pub created_at: String,
}

pub async fn history(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<Vec<TransactionDto>>, ApiError> {
    let txs = state
        .wallet_history
        .history(&guild_id, &user_id, q.limit, q.offset)
        .await?;
    Ok(Json(
        txs.into_iter()
            .map(|t| TransactionDto {
                id: t.id.to_string(),
                amount: t.amount,
                balance_after: t.balance_after,
                source: t.source,
                description: t.description,
                reason: t.reason,
                created_at: t.created_at.to_rfc3339(),
            })
            .collect(),
    ))
}

// ── Leaderboard ──

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    pub limit: Option<i64>,
}

pub async fn leaderboard(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(q): Query<LeaderboardQuery>,
) -> Result<Json<Vec<WalletDto>>, ApiError> {
    let wallets = state
        .wallet_leaderboard
        .leaderboard(&guild_id, q.limit)
        .await?;
    Ok(Json(wallets.into_iter().map(WalletDto::from).collect()))
}
// Handlers HTTP des portefeuilles NEXUS. Toutes les lectures et écritures
// sont limitées à une guilde et les transferts doivent rester transactionnels.
