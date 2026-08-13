//! Handlers HTTP des salons de discussion lies a une review et de leur
//! transcript persistant, plus le nettoyage des cartes expirees.

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::validation;
use crate::sentinel::bootstrap::state::ModerationState;

use super::dto::DiscussionChannelDto;
use super::dto::DiscussionMessageDto;

/// GET /api/automod/reviews/{review_id}/discussion
/// Retourne le salon de discussion existant (ou `null`).
pub async fn get_discussion(
    State(state): State<ModerationState>,
    Path(review_id): Path<String>,
) -> Result<Json<Option<DiscussionChannelDto>>, ApiError> {
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    let existing = state.automod_reviews_uc.get_discussion(id).await?;
    Ok(Json(
        existing.map(|d| DiscussionChannelDto::build(d, false)),
    ))
}

/// DELETE /api/automod/reviews/{review_id}/discussion
/// Purge l'enregistrement du salon (le salon Discord a ete supprime a la
/// main) afin de pouvoir en rouvrir un neuf. Idempotent.
pub async fn delete_discussion(
    State(state): State<ModerationState>,
    Path(review_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    state.automod_reviews_uc.delete_discussion(id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct OpenDiscussionBody {
    pub guild_id: String,
    pub channel_id: String,
    pub opened_by_id: String,
    pub opened_by_name: String,
    // Faits Discord du demandeur (la decision d'acces est prise par le domaine).
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub has_moderate_members: bool,
    #[serde(default)]
    pub has_manage_messages: bool,
    #[serde(default)]
    pub has_mod_role: bool,
}

/// POST /api/automod/reviews/{review_id}/discussion
/// Enregistre (idempotent) un salon de discussion apres application de la
/// regle d'acces (`can_open_discussion`). `403` si non autorise.
pub async fn open_discussion(
    State(state): State<ModerationState>,
    Path(review_id): Path<String>,
    Json(body): Json<OpenDiscussionBody>,
) -> Result<Json<DiscussionChannelDto>, ApiError> {
    use platform_core::sentinel::domain::entities::moderation::review::automod::ModeratorFacts;
    use platform_core::sentinel::ports::inbound::moderation::manage_automod_reviews::OpenDiscussionCommand;

    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;

    let (discussion, created) = state
        .automod_reviews_uc
        .open_discussion(OpenDiscussionCommand {
            review_id: id,
            guild_id: body.guild_id.clone(),
            channel_id: body.channel_id,
            opened_by_id: body.opened_by_id.clone(),
            opened_by_name: body.opened_by_name,
            requester: ModeratorFacts {
                is_admin: body.is_admin,
                has_moderate_members: body.has_moderate_members,
                has_manage_messages: body.has_manage_messages,
                has_mod_role: body.has_mod_role,
                has_admin_role: false,
            },
        })
        .await?;

    if created {
        state.broadcaster.broadcast(
            "automod_discussion_opened",
            serde_json::json!({
                "review_id": review_id,
                "guild_id": &body.guild_id,
                "channel_id": &discussion.channel_id,
                "opened_by_id": &body.opened_by_id,
            }),
        );
    }

    Ok(Json(DiscussionChannelDto::build(discussion, created)))
}

// ── Transcript du salon de discussion (trace persistante) ──

#[derive(Debug, Deserialize)]
pub struct DiscussionMessageIn {
    pub discord_message_id: String,
    pub author_id: String,
    #[serde(default)]
    pub author_name: String,
    #[serde(default)]
    pub author_is_bot: bool,
    #[serde(default)]
    pub content: String,
    /// RFC3339.
    pub sent_at: String,
}

#[derive(Debug, Deserialize)]
pub struct AppendDiscussionMessagesBody {
    pub messages: Vec<DiscussionMessageIn>,
}

/// POST /api/automod/reviews/{review_id}/discussion/messages
/// Persiste un lot de messages du salon (appele par le bot a l'archivage).
pub async fn append_discussion_messages(
    State(state): State<ModerationState>,
    Path(review_id): Path<String>,
    Json(body): Json<AppendDiscussionMessagesBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use platform_core::sentinel::domain::entities::moderation::review::automod::DiscussionMessage;
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;

    let messages: Vec<DiscussionMessage> = body
        .messages
        .into_iter()
        .filter_map(|m| {
            let sent_at = chrono::DateTime::parse_from_rfc3339(&m.sent_at)
                .ok()?
                .with_timezone(&chrono::Utc);
            Some(DiscussionMessage {
                review_id: id,
                discord_message_id: m.discord_message_id,
                author_id: m.author_id,
                author_name: m.author_name,
                author_is_bot: m.author_is_bot,
                content: m.content,
                sent_at,
            })
        })
        .collect();

    let inserted = state
        .automod_reviews_uc
        .append_discussion_messages(messages)
        .await?;
    Ok(Json(serde_json::json!({ "inserted": inserted })))
}

#[derive(Debug, Deserialize)]
pub struct CleanupCardsQuery {
    /// Age minimum (jours) d'une carte close pour etre supprimee. Defaut 30.
    pub days: Option<i64>,
}

/// POST /api/automod/cleanup-expired-cards — appele par le worker (24h).
/// Pour chaque carte de review CLOSE (applied|ignored) resolue depuis plus de
/// `days` jours et encore mappee a un message Discord : broadcast un event
/// `automod_card_expired` (le bot supprime le message) et retire le mapping.
/// La review + le transcript restent en DB (trace web conservee).
pub async fn cleanup_expired_cards(
    State(state): State<ModerationState>,
    Query(q): Query<CleanupCardsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Le use case trouve les cartes expirees et retire leur mapping ; le
    // handler ne fait que diffuser l'event d'expiration (le bot supprime le
    // message Discord correspondant).
    // F5 : d'abord, les reviews 'decided' jamais finalisees par un admin depuis
    // plus de 48h -> passees en 'ignored' (le verdict lapse) pour ne plus rester
    // bloquees. Leurs cartes sont nettoyees comme les autres.
    let mut cards = state
        .automod_reviews_uc
        .expire_stale_decided_reviews(48, 200)
        .await?;
    cards.extend(
        state
            .automod_reviews_uc
            .expired_review_cards(q.days.unwrap_or(30), 200)
            .await?,
    );

    let count = cards.len() as u32;
    for c in &cards {
        state.broadcaster.broadcast(
            "automod_card_expired",
            serde_json::json!({
                "action_id": c.action_id.to_string(),
                "channel_id": c.channel_id,
                "message_id": c.message_id,
            }),
        );
    }
    Ok(Json(serde_json::json!({ "expired": count })))
}

/// GET /api/automod/reviews/{review_id}/discussion/messages
/// Liste le transcript (trace) pour affichage web.
pub async fn list_discussion_messages(
    State(state): State<ModerationState>,
    Path(review_id): Path<String>,
) -> Result<Json<Vec<DiscussionMessageDto>>, ApiError> {
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    let msgs = state
        .automod_reviews_uc
        .list_discussion_messages(id)
        .await?;
    let dtos = msgs
        .into_iter()
        .map(|m| DiscussionMessageDto {
            discord_message_id: m.discord_message_id,
            author_id: m.author_id,
            author_name: m.author_name,
            author_is_bot: m.author_is_bot,
            content: m.content,
            sent_at: m.sent_at.to_rfc3339(),
        })
        .collect();
    Ok(Json(dtos))
}
