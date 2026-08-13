//! DTOs publics du module Automod et leurs conversions depuis le domaine.

use serde::Serialize;

use platform_core::sentinel::domain::entities::moderation::review::automod::AutomodReview;
use platform_core::sentinel::domain::entities::system::discord_ids::ChannelId;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::MessageId;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;

/// DTO public d'une carte de review automod.
#[derive(Debug, Serialize)]
pub struct AutomodReviewDto {
    pub id: String,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub user_id: UserId,
    pub user_name: String,
    pub content_preview: String,
    pub suggested_action: String,
    pub score: f64,
    pub reason: String,
    pub flags: serde_json::Value,
    pub status: String,
    pub applied_action: Option<String>,
    pub resolved_by_id: Option<String>,
    pub resolved_by_name: Option<String>,
    pub resolved_source: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
    pub voting_deadline: Option<String>,
    pub decided_action: Option<String>,
    pub quorum_met: bool,
    pub decided_at: Option<String>,
    pub incident_count: i32,
    pub cumulative_score: f64,
    pub incidents: serde_json::Value,
    /// True si ce POST a ete agrege dans une carte existante (pas une creation).
    pub merged: bool,
    /// Salon de discussion lie a cette review (si ouvert), pour la page web.
    pub discussion_channel_id: Option<String>,
}

impl From<AutomodReview> for AutomodReviewDto {
    fn from(r: AutomodReview) -> Self {
        Self {
            id: r.id.to_string(),
            guild_id: r.guild_id,
            channel_id: r.channel_id,
            message_id: r.message_id,
            user_id: r.user_id,
            user_name: r.user_name,
            content_preview: r.content_preview,
            suggested_action: r.suggested_action,
            score: r.score,
            reason: r.reason,
            flags: r.flags,
            status: r.status,
            applied_action: r.applied_action,
            resolved_by_id: r.resolved_by_id,
            resolved_by_name: r.resolved_by_name,
            resolved_source: r.resolved_source,
            created_at: r.created_at.to_rfc3339(),
            resolved_at: r.resolved_at.map(|d| d.to_rfc3339()),
            voting_deadline: r.voting_deadline.map(|d| d.to_rfc3339()),
            decided_action: r.decided_action,
            quorum_met: r.quorum_met,
            decided_at: r.decided_at.map(|d| d.to_rfc3339()),
            incident_count: r.incident_count,
            cumulative_score: r.cumulative_score,
            incidents: r.incidents,
            merged: false,
            discussion_channel_id: None,
        }
    }
}

/// DTO d'un vote individuel.
#[derive(Debug, Serialize)]
pub struct ReviewVoteDto {
    pub voter_id: String,
    pub voter_name: String,
    pub vote_action: String,
}

impl From<platform_core::sentinel::domain::entities::moderation::review::automod::ReviewVote>
    for ReviewVoteDto
{
    fn from(
        v: platform_core::sentinel::domain::entities::moderation::review::automod::ReviewVote,
    ) -> Self {
        Self {
            voter_id: v.voter_id,
            voter_name: v.voter_name,
            vote_action: v.vote_action,
        }
    }
}

/// DTO d'un salon de discussion lie a une review.
#[derive(Debug, Serialize)]
pub struct DiscussionChannelDto {
    pub id: String,
    pub review_id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub opened_by_id: String,
    pub opened_by_name: String,
    pub created_at: String,
    /// True si ce POST vient de creer le salon (false = il existait deja).
    pub created: bool,
}

impl DiscussionChannelDto {
    pub(super) fn build(
        d: platform_core::sentinel::domain::entities::moderation::review::automod::DiscussionChannel,
        created: bool,
    ) -> Self {
        Self {
            id: d.id.to_string(),
            review_id: d.review_id.to_string(),
            guild_id: d.guild_id,
            channel_id: d.channel_id,
            opened_by_id: d.opened_by_id,
            opened_by_name: d.opened_by_name,
            created_at: d.created_at.to_rfc3339(),
            created,
        }
    }
}

/// DTO d'un message de transcript du salon de discussion.
#[derive(Debug, Serialize)]
pub struct DiscussionMessageDto {
    pub discord_message_id: String,
    pub author_id: String,
    pub author_name: String,
    pub author_is_bot: bool,
    pub content: String,
    pub sent_at: String,
}
