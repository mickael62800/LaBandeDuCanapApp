//! Handlers pour la persistance des donnees fire-and-forget des bots.
//! Ces endpoints sont appeles par les bots pour persister des donnees
//! qui etaient auparavant uniquement en memoire (DashMap).
//!
//! Approche pragmatique : sqlx direct depuis le handler (pas de full hexagonal)
//! car ces endpoints sont simples et fire-and-forget cote bot.

use crate::sentinel::adapters::inbound::http::extractors::{ValidatedGuild, ValidatedGuildUser};
use axum::extract::Path;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;

use tracing::warn;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::helpers::ok_response;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::adapters::inbound::http::validation;
use crate::sentinel::bootstrap::state::BotPersistenceState;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::RoleId;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;

// ═══════════════════════════════════════════════════
// Name History (Audit Bot)
// ═══════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CreateNameHistoryDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub old_name: String,
    pub new_name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct NameHistoryEntryDto {
    pub id: String,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub old_name: String,
    pub new_name: String,
    pub created_at: String,
}

/// GET /api/name-history/{guild_id}/{user_id}
///
/// Liste l'historique de pseudos d'un utilisateur, deduit des audit_logs
/// `member_nickname_history`. Trie par created_at desc, max 50 entrees.
/// Respect de l'archi hexagonale : passe par `audit_logs_uc.list()`.
pub async fn list_name_history(
    State(state): State<BotPersistenceState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<Vec<NameHistoryEntryDto>>, ApiError> {
    use platform_core::sentinel::ports::inbound::audit::manage_audit_logs::AuditLogFilters;
    let logs = state
        .audit_logs_uc
        .list(
            Some(&guild_id),
            AuditLogFilters {
                event_type: Some(
                    platform_core::sentinel::domain::entities::audit::audit_log::AUDIT_EVENT_MEMBER_NICKNAME_HISTORY.to_string(),
                ),
                target_id: Some(user_id.clone()),
                limit: 50,
                ..Default::default()
            },
        )
        .await?;

    let entries = logs
        .into_iter()
        .map(|l| NameHistoryEntryDto {
            id: l.id.to_string(),
            guild_id: l.guild_id,
            user_id: l.target_id.clone().unwrap_or_default().into(),
            old_name: l
                .details
                .get("old_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            new_name: l
                .details
                .get("new_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            created_at: l.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(entries))
}

/// POST /api/name-history
pub async fn create_name_history(
    State(state): State<BotPersistenceState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<CreateNameHistoryDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_guild_user_path(&dto.guild_id, &dto.user_id).map_err(ApiError)?;

    state.audit_logs_uc.create(platform_core::sentinel::ports::inbound::audit::manage_audit_logs::CreateAuditLogCommand {
        guild_id: dto.guild_id,
        event_type: platform_core::sentinel::domain::entities::audit::audit_log::AUDIT_EVENT_MEMBER_NICKNAME_HISTORY.into(),
        actor_id: None,
        actor_name: None,
        target_id: Some(dto.user_id.clone().into()),
        target_name: Some(dto.new_name.clone()),
        channel_id: None,
        channel_name: None,
        details: serde_json::json!({
            "old_name": dto.old_name,
            "new_name": dto.new_name,
        }),
    }).await
    .inspect_err(|e| warn!(error = %e, "Echec insert name_history"))
    .ok();

    Ok(ok_response())
}

// ═══════════════════════════════════════════════════
// Streaks (Progression Bot)
// ═══════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct UpdateStreakDto {
    pub streak_current: i32,
    pub streak_best: i32,
    pub streak_last_day: i32,
    pub streak_last_year: i32,
}

/// PATCH /api/levels/{guild_id}/{user_id}/streak
pub async fn update_streak(
    State(state): State<BotPersistenceState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
    Json(dto): Json<UpdateStreakDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .bot_persistence_uc
        .update_streak(
            &guild_id,
            &user_id,
            dto.streak_current,
            dto.streak_best,
            dto.streak_last_day,
            dto.streak_last_year,
        )
        .await
        .inspect_err(
            |e| warn!(error = %e, guild_id = %guild_id, user_id = %user_id, "Echec update streak"),
        )
        .ok();

    Ok(ok_response())
}

// ═══════════════════════════════════════════════════
// SLA Tickets (Ticket Bot)
// ═══════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct UpdateTicketSlaDto {
    pub first_response_at: Option<String>,
    pub resolved_at: Option<String>,
    pub satisfaction_rating: Option<i32>,
}

/// PATCH /api/tickets/{id}/sla
pub async fn update_ticket_sla(
    State(state): State<BotPersistenceState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateTicketSlaDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let uuid = validation::parse_uuid("id", &id).map_err(ApiError)?;
    state
        .tickets_uc
        .update_sla(
            uuid,
            dto.first_response_at.as_deref(),
            dto.resolved_at.as_deref(),
            dto.satisfaction_rating,
        )
        .await
        .inspect_err(|e| warn!(error = %e, ticket_id = %id, "Echec update_sla"))
        .ok();

    Ok(ok_response())
}

// ═══════════════════════════════════════════════════
// Sponsorships (Community Bot)
// ═══════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CreateSponsorshipDto {
    pub guild_id: GuildId,
    pub sponsor_id: String,
    pub sponsored_id: String,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SponsorshipRow {
    pub id: sqlx::types::Uuid,
    pub guild_id: GuildId,
    pub sponsor_id: String,
    pub sponsored_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// POST /api/sponsorships
pub async fn create_sponsorship(
    State(state): State<BotPersistenceState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<CreateSponsorshipDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("sponsor_id", &dto.sponsor_id).map_err(ApiError)?;
    validation::validate_discord_id("sponsored_id", &dto.sponsored_id).map_err(ApiError)?;

    // C4 — Gate user : moderator+ requis pour creer un parrainage.
    // Pass-through pour les appels bot-internal (user absent).

    state
        .sponsorship_repo
        .create(&dto.guild_id, &dto.sponsor_id, &dto.sponsored_id)
        .await
        .inspect_err(|e| warn!(error = %e, guild_id = %dto.guild_id, "Echec insert sponsorship"))
        .ok();

    Ok(ok_response())
}

/// GET /api/sponsorships/{guild_id}
pub async fn list_sponsorships(
    State(state): State<BotPersistenceState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<
    Json<Vec<platform_core::sentinel::ports::outbound::community::sponsorship_repository::Sponsorship>>,
    ApiError,
>{
    // Validation

    let entries = state
        .sponsorship_repo
        .list(&guild_id)
        .await
        .unwrap_or_else(|e| {
            warn!(error = %e, guild_id = %guild_id, "Echec list sponsorships");
            vec![]
        });

    Ok(Json(entries))
}

// ═══════════════════════════════════════════════════
// Temp Roles (Community Bot)
// ═══════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CreateTempRoleDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub role_id: RoleId,
    pub expires_at: String,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct TempRoleRow {
    pub id: sqlx::types::Uuid,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub role_id: RoleId,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// POST /api/temp-roles
pub async fn create_temp_role(
    State(state): State<BotPersistenceState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<CreateTempRoleDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &dto.user_id).map_err(ApiError)?;
    validation::validate_discord_id("role_id", &dto.role_id).map_err(ApiError)?;

    // C5 — Gate user : moderator+ requis pour assigner un role temporaire.

    state
        .temp_role_repo
        .create(&dto.guild_id, &dto.user_id, &dto.role_id, &dto.expires_at)
        .await
        .inspect_err(|e| warn!(error = %e, guild_id = %dto.guild_id, "Echec insert temp_role"))
        .ok();

    Ok(ok_response())
}

/// GET /api/temp-roles/{guild_id}
pub async fn list_temp_roles(
    State(state): State<BotPersistenceState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<
    Json<Vec<platform_core::sentinel::ports::outbound::community::temp_role_repository::TempRole>>,
    ApiError,
> {
    // Validation

    let entries = state
        .temp_role_repo
        .list_active(&guild_id)
        .await
        .unwrap_or_else(|e| {
            warn!(error = %e, guild_id = %guild_id, "Echec list temp_roles");
            vec![]
        });

    Ok(Json(entries))
}

/// DELETE /api/temp-roles/{guild_id}/{user_id}/{role_id}
pub async fn delete_temp_role(
    State(state): State<BotPersistenceState>,
    _user: Option<Extension<WebUser>>,
    Path((guild_id, user_id, role_id)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &user_id).map_err(ApiError)?;
    validation::validate_discord_id("role_id", &role_id).map_err(ApiError)?;

    // Phase 7 B — Gate user : moderator+ requis depuis le desktop. Les bots
    // (community-bot qui consume l'event temp_role_expire) appellent sans
    // X-Discord-Token → pass-through non-breaking.

    state
        .temp_role_repo
        .delete(&guild_id, &user_id, &role_id)
        .await
        .inspect_err(|e| warn!(error = %e, guild_id = %guild_id, "Echec delete temp_role"))
        .ok();

    Ok(ok_response())
}

// ═══════════════════════════════════════════════════
// Pending Moderation Actions (Moderation Bot - Mode Apprenti)
// ═══════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CreatePendingActionDto {
    pub guild_id: GuildId,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub gravity: Option<String>,
    pub duration: Option<i64>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct PendingActionRow {
    pub id: sqlx::types::Uuid,
    pub guild_id: GuildId,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub gravity: Option<String>,
    pub duration: Option<i64>,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// POST /api/moderation/pending
pub async fn create_pending_action(
    State(state): State<BotPersistenceState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<CreatePendingActionDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_moderation_action(
        &dto.guild_id,
        &dto.moderator_id,
        &dto.target_id,
        &dto.reason,
        &dto.action_type,
    )
    .map_err(ApiError)?;

    match state
        .pending_action_repo
        .create(
            &dto.guild_id,
            &dto.moderator_id,
            &dto.moderator_name,
            &dto.target_id,
            &dto.target_name,
            &dto.action_type,
            &dto.reason,
            dto.gravity.as_deref(),
            dto.duration,
        )
        .await
    {
        Ok(id) => Ok(Json(serde_json::json!({ "id": id.to_string() }))),
        Err(e) => {
            warn!(error = %e, guild_id = %dto.guild_id, "Echec creation pending_action");
            Ok(ok_response())
        }
    }
}

/// GET /api/moderation/pending/{guild_id}
pub async fn list_pending_actions(
    State(state): State<BotPersistenceState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<
    Json<Vec<platform_core::sentinel::ports::outbound::moderation::pending_action_repository::PendingAction>>,
    ApiError,
>{
    // Validation

    let entries = state
        .pending_action_repo
        .list_pending(&guild_id)
        .await
        .unwrap_or_else(|e| {
            warn!(error = %e, guild_id = %guild_id, "Echec list pending_mod_actions");
            vec![]
        });

    Ok(Json(entries))
}

#[derive(Debug, Deserialize)]
pub struct ResolvePendingActionDto {
    pub status: String,
    pub reviewed_by: String,
}

/// PATCH /api/moderation/pending/{id}
pub async fn resolve_pending_action(
    State(state): State<BotPersistenceState>,
    // TODO(secu, ex-H10) : la reverification � lookup du guild_id de l'action
    // pending puis gate Moderator+ � n'est PAS implementee. Seuls les
    // middlewares du routeur protegent cette route. L'ancien
    // `if user.is_some() {}` ne verifiait rien.
    _user: Option<Extension<WebUser>>,
    Path(id): Path<String>,
    Json(dto): Json<ResolvePendingActionDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let uuid = validation::parse_uuid("id", &id).map_err(ApiError)?;

    state
        .pending_action_repo
        .resolve(uuid, &dto.status, &dto.reviewed_by)
        .await
        .inspect_err(|e| warn!(error = %e, action_id = %id, "Echec resolution pending_action"))
        .ok();

    Ok(ok_response())
}

#[cfg(test)]
#[path = "tests/bot_persistence.rs"]
mod tests;
