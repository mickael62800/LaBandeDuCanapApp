use crate::nexus::{adapters::inbound::http::handlers::ApiError, bootstrap::AppState};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
#[derive(Deserialize)]
pub struct ProfileQuery {
    pub username: Option<String>,
}
#[derive(Serialize)]
pub struct ProfileDto {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub class: String,
    pub level: i32,
    pub xp: i64,
    pub atk: i32,
    pub def: i32,
    pub hp_current: i32,
    pub hp_max: i32,
    pub coins: i64,
    pub stat_points: i32,
    pub title: String,
    pub total_wins: i32,
    pub total_losses: i32,
    pub total_draws: i32,
    pub total_stolen: i64,
    pub cowardice_count: i32,
    pub chaos_events: i32,
}
pub async fn profile(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Query(q): Query<ProfileQuery>,
) -> Result<Json<ProfileDto>, ApiError> {
    let p = state
        .coussin_profile
        .profile(&guild_id, &user_id, q.username.as_deref().unwrap_or(""))
        .await?;
    Ok(Json(ProfileDto {
        guild_id: p.guild_id,
        user_id: p.user_id,
        username: p.username,
        class: p.class.as_str().into(),
        level: p.level,
        xp: p.xp,
        atk: p.atk,
        def: p.def,
        hp_current: p.hp_current,
        hp_max: p.hp_max,
        coins: p.coins,
        stat_points: p.stat_points,
        title: p.title,
        total_wins: p.total_wins,
        total_losses: p.total_losses,
        total_draws: p.total_draws,
        total_stolen: p.total_stolen,
        cowardice_count: p.cowardice_count,
        chaos_events: p.chaos_events,
    }))
}

#[derive(Deserialize)]
pub struct RankingQuery {
    pub limit: Option<i64>,
}

/// GET /api/coussin/{guild_id}/classement — supervision cote web.
///
/// Lecture seule : ne cree aucun profil, contrairement a `profile` qui
/// materialise le joueur au premier appel.
pub async fn ranking(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(q): Query<RankingQuery>,
) -> Result<Json<Vec<ProfileDto>>, ApiError> {
    let list = state
        .coussin_profile
        .ranking(&guild_id, q.limit.unwrap_or(50))
        .await?;
    Ok(Json(list.into_iter().map(profile_dto).collect()))
}

#[derive(Deserialize)]
pub struct ClassRequest {
    pub username: String,
    pub class: String,
}

fn profile_dto(
    p: platform_core::nexus::ports::outbound::coussin_repository::CoussinProfile,
) -> ProfileDto {
    ProfileDto {
        guild_id: p.guild_id,
        user_id: p.user_id,
        username: p.username,
        class: p.class.as_str().into(),
        level: p.level,
        xp: p.xp,
        atk: p.atk,
        def: p.def,
        hp_current: p.hp_current,
        hp_max: p.hp_max,
        coins: p.coins,
        stat_points: p.stat_points,
        title: p.title,
        total_wins: p.total_wins,
        total_losses: p.total_losses,
        total_draws: p.total_draws,
        total_stolen: p.total_stolen,
        cowardice_count: p.cowardice_count,
        chaos_events: p.chaos_events,
    }
}
pub async fn choose_class(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(req): Json<ClassRequest>,
) -> Result<Json<ProfileDto>, ApiError> {
    Ok(Json(profile_dto(
        state
            .coussin_profile
            .choose_class(&guild_id, &user_id, &req.username, &req.class)
            .await?,
    )))
}
#[derive(Deserialize)]
pub struct TrainRequest {
    pub username: String,
    pub stat: String,
}
pub async fn train(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(req): Json<TrainRequest>,
) -> Result<Json<ProfileDto>, ApiError> {
    Ok(Json(profile_dto(
        state
            .coussin_profile
            .train(&guild_id, &user_id, &req.username, &req.stat)
            .await?,
    )))
}

#[derive(Serialize)]
pub struct InventoryDto {
    pub item_key: String,
    pub quantity: i32,
}
pub async fn inventory(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Vec<InventoryDto>>, ApiError> {
    Ok(Json(
        state
            .coussin_inventory
            .inventory(&guild_id, &user_id)
            .await?
            .into_iter()
            .map(|item| InventoryDto {
                item_key: item.item_key,
                quantity: item.quantity,
            })
            .collect(),
    ))
}
#[derive(Deserialize)]
pub struct BuyItemRequest {
    pub item_key: String,
}
pub async fn buy_item(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(req): Json<BuyItemRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let balance = state
        .coussin_inventory
        .buy(&guild_id, &user_id, &req.item_key)
        .await?;
    Ok(Json(serde_json::json!({"balance_after": balance})))
}
pub async fn buy_insurance(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let insurance = state.coussin_insurance.buy(&guild_id, &user_id).await?;
    Ok(Json(
        serde_json::json!({"is_scam": insurance.is_scam, "expires_at": insurance.expires_at}),
    ))
}
pub async fn insurance(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let active = state.coussin_insurance.active(&guild_id, &user_id).await?;
    Ok(Json(
        serde_json::json!({"active": active.is_some(), "is_scam": active.as_ref().map(|i| i.is_scam), "expires_at": active.map(|i| i.expires_at)}),
    ))
}
#[derive(Deserialize)]
pub struct StealRequest {
    pub thief_name: String,
    pub victim_id: String,
    pub victim_name: String,
}
pub async fn steal(
    State(state): State<AppState>,
    Path((guild_id, thief_id)): Path<(String, String)>,
    Json(req): Json<StealRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let profile = state
        .coussin_profile
        .profile(&guild_id, &thief_id, &req.thief_name)
        .await?;
    state
        .coussin_profile
        .profile(&guild_id, &req.victim_id, &req.victim_name)
        .await?;
    let result = state
        .coussin_steal
        .steal(
            &guild_id,
            &thief_id,
            &req.victim_id,
            profile.class == platform_core::nexus::domain::entities::coussin::PlayerClass::Piegeur,
        )
        .await?;
    Ok(Json(
        serde_json::json!({"success":result.success,"amount":result.amount}),
    ))
}
#[derive(Deserialize)]
pub struct PrimeRequest {
    pub target_id: String,
    pub target_name: String,
    pub placer_name: String,
    pub amount: i64,
}
pub async fn place_prime(
    State(state): State<AppState>,
    Path((guild_id, placer_id)): Path<(String, String)>,
    Json(req): Json<PrimeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .coussin_profile
        .profile(&guild_id, &placer_id, &req.placer_name)
        .await?;
    state
        .coussin_profile
        .profile(&guild_id, &req.target_id, &req.target_name)
        .await?;
    state
        .coussin_prime
        .place(
            &guild_id,
            &req.target_id,
            &req.target_name,
            &placer_id,
            &req.placer_name,
            req.amount,
        )
        .await?;
    Ok(Json(serde_json::json!({"ok":true})))
}
#[derive(Deserialize)]
pub struct BetRequest {
    pub combat_id: uuid::Uuid,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount: i64,
}
pub async fn place_bet(
    State(state): State<AppState>,
    Path((guild_id, bettor_id)): Path<(String, String)>,
    Json(req): Json<BetRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .coussin_bet
        .place(
            &guild_id,
            req.combat_id,
            &bettor_id,
            &req.bettor_name,
            &req.backed_id,
            req.amount,
        )
        .await?;
    Ok(Json(serde_json::json!({"ok":true})))
}

#[derive(Deserialize)]
pub struct ChallengeRequest {
    pub channel_id: String,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
}
#[derive(Serialize)]
pub struct ChallengeDto {
    pub id: String,
    pub status: String,
    pub mise: i64,
}
pub async fn challenge(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(req): Json<ChallengeRequest>,
) -> Result<Json<ChallengeDto>, ApiError> {
    let combat = state
        .coussin_combat
        .challenge(
            &guild_id,
            &req.channel_id,
            &req.attacker_id,
            &req.attacker_name,
            &req.defender_id,
            &req.defender_name,
            req.mise,
        )
        .await?;
    Ok(Json(ChallengeDto {
        id: combat.id.to_string(),
        status: combat.status,
        mise: combat.mise,
    }))
}

#[derive(Deserialize)]
pub struct DefenderRequest {
    pub defender_id: String,
}
pub async fn accept(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<DefenderRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let ok = state.coussin_combat.accept(id, &req.defender_id).await?;
    Ok(Json(serde_json::json!({"ok": ok})))
}
pub async fn refuse(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<DefenderRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let ok = state.coussin_combat.refuse(id, &req.defender_id).await?;
    Ok(Json(serde_json::json!({"ok": ok})))
}
pub async fn resolve(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let ok = state.coussin_combat.resolve(id).await?;
    Ok(Json(serde_json::json!({"ok":ok})))
}

#[derive(Debug, Serialize)]
pub struct CombatDto {
    pub id: String,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>,
    pub defender_roll: Option<i32>,
    /// Evenement chaotique survenu pendant le combat, s'il y en a eu un.
    pub chaos_event: Option<String>,
    pub special_attack: Option<String>,
    /// Recit du combat, tel qu'il a ete poste sur Discord.
    pub result_message: Option<String>,
    pub coins_transferred: i64,
    pub resolved_at: Option<String>,
}

/// GET /api/coussin/{guild_id}/{user_id}/combats?limit=
pub async fn combat_history(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<Vec<CombatDto>>, ApiError> {
    let combats = state
        .coussin_profile
        .combat_history(&guild_id, &user_id, q.limit.unwrap_or(10))
        .await?;

    Ok(Json(
        combats
            .into_iter()
            .map(|c| CombatDto {
                id: c.id.to_string(),
                attacker_id: c.attacker_id,
                attacker_name: c.attacker_name,
                defender_id: c.defender_id,
                defender_name: c.defender_name,
                mise: c.mise,
                winner_id: c.winner_id,
                attacker_roll: c.attacker_roll,
                defender_roll: c.defender_roll,
                chaos_event: c.chaos_event,
                special_attack: c.special_attack,
                result_message: c.result_message,
                coins_transferred: c.coins_transferred,
                resolved_at: c.resolved_at.map(|d| d.to_rfc3339()),
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
}
// Handlers HTTP du Coussin Piégé. Les services du domaine appliquent les
// cooldowns, les règles d'équilibrage et les vérifications de portefeuille.
