//! Implementation gRPC du `AutomodReviewService` (tranche 1 : review-core).
//!
//! Wrappe `ManageAutomodReviewsUseCase`. Remplace les endpoints HTTP
//! `POST/GET /api/automod/reviews...` appeles par automod-bot (post, vote,
//! ignore, reopen, finalize). Le chemin gRPC est TOUJOURS le chemin bot de
//! confiance : les `ModeratorFacts` viennent du corps (le bot passe les vraies
//! permissions gateway), il n'y a pas de `WebUser` a arbitrer ici.

use std::sync::Arc;

use platform_proto::sentinel::automod_review::v1 as proto;
use platform_proto::sentinel::automod_review::v1::automod_review_service_server::AutomodReviewService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use crate::sentinel::adapters::inbound::http::handlers::moderation::automod::reviews::log_review_sanction;
use crate::sentinel::adapters::outbound::ws::broadcaster::EventBroadcaster;
use platform_core::sentinel::domain::entities::moderation::review::automod::AutomodReview;
use platform_core::sentinel::domain::entities::moderation::review::automod::DiscussionMessage;
use platform_core::sentinel::domain::entities::moderation::review::automod::ModeratorFacts;
use platform_core::sentinel::domain::entities::moderation::review::automod::NewAutomodReview;
use platform_core::sentinel::domain::entities::moderation::review::automod::ReviewVote;
use platform_core::sentinel::domain::entities::moderation::review::automod::SuggestedAction;
use platform_core::sentinel::ports::inbound::moderation::manage_automod_reviews::CastVoteCommand;
use platform_core::sentinel::ports::inbound::moderation::manage_automod_reviews::CloseIgnoredCommand;
use platform_core::sentinel::ports::inbound::moderation::manage_automod_reviews::ManageAutomodReviewsUseCase;
use platform_core::sentinel::ports::inbound::moderation::manage_automod_reviews::OpenDiscussionCommand;
use platform_core::sentinel::ports::inbound::moderation::manage_automod_reviews::ReopenReviewCommand;
use platform_core::sentinel::ports::inbound::moderation::manage_automod_reviews::ResolveAutomodReviewCommand;
use platform_core::sentinel::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use platform_core::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;

pub struct AutomodReviewGrpc {
    pub reviews_uc: Arc<dyn ManageAutomodReviewsUseCase>,
    /// Journalisation de la sanction de membre a la finalisation (resolve),
    /// reutilise le meme helper que le handler HTTP.
    pub moderation_uc: Arc<dyn ManageModerationUseCase>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    pub broadcaster: Arc<EventBroadcaster>,
}

#[tonic::async_trait]
impl AutomodReviewService for AutomodReviewGrpc {
    async fn create_review(
        &self,
        request: Request<proto::CreateReviewRequest>,
    ) -> Result<Response<proto::AutomodReview>, Status> {
        let req = request.into_inner();
        let suggested = SuggestedAction::from_str(&req.suggested_action).ok_or_else(|| {
            Status::invalid_argument(format!(
                "suggested_action invalide : {}",
                req.suggested_action
            ))
        })?;
        let flags = parse_json_or_empty_object(&req.flags_json)?;
        let voting_deadline = req
            .voting_deadline
            .as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&chrono::Utc));

        let (review, merged) = self
            .reviews_uc
            .create_or_merge(
                NewAutomodReview {
                    guild_id: req.guild_id.into(),
                    channel_id: req.channel_id.into(),
                    message_id: req.message_id.into(),
                    user_id: req.user_id.into(),
                    user_name: req.user_name,
                    content_preview: req.content_preview,
                    suggested_action: suggested,
                    score: req.score,
                    reason: req.reason,
                    flags,
                    voting_deadline,
                    sanction_logged: req.already_sanctioned,
                },
                req.aggregate,
                req.aggregate_window_minutes.unwrap_or(60),
            )
            .await
            .map_err(domain_to_status)?;

        self.broadcaster.broadcast(
            if merged {
                "automod_review_updated"
            } else {
                "automod_review_created"
            },
            serde_json::json!({
                "review_id": review.id.to_string(),
                "guild_id": review.guild_id.as_str(),
                "user_id": review.user_id.as_str(),
                "merged": merged,
            }),
        );

        Ok(Response::new(review_to_proto(review, merged, None)))
    }

    async fn get_review(
        &self,
        request: Request<proto::GetReviewRequest>,
    ) -> Result<Response<proto::AutomodReview>, Status> {
        let id = parse_uuid(&request.into_inner().review_id)?;
        match self.reviews_uc.get(id).await.map_err(domain_to_status)? {
            Some(review) => {
                let discussion = self.discussion_channel_id(id).await;
                Ok(Response::new(review_to_proto(review, false, discussion)))
            }
            None => Err(Status::not_found("review introuvable")),
        }
    }

    async fn find_review_by_message(
        &self,
        request: Request<proto::FindReviewByMessageRequest>,
    ) -> Result<Response<proto::FindReviewByMessageResponse>, Status> {
        let req = request.into_inner();
        let review = self
            .reviews_uc
            .find_by_message_id(&req.guild_id, &req.message_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::FindReviewByMessageResponse {
            review: review.map(|r| review_to_proto(r, false, None)),
        }))
    }

    async fn resolve_review(
        &self,
        request: Request<proto::ResolveReviewRequest>,
    ) -> Result<Response<proto::AutomodReview>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.review_id)?;
        let review = self
            .reviews_uc
            .resolve(ResolveAutomodReviewCommand {
                review_id: id,
                applied_action: req.applied_action.clone(),
                resolved_by_id: req.resolved_by_id.clone(),
                resolved_by_name: req.resolved_by_name.clone(),
                resolved_source: "discord".into(),
                requester: req.requester.map(facts_from_proto),
            })
            .await
            .map_err(domain_to_status)?;

        // Tracabilite : journalise la sanction de membre cote serveur (meme
        // helper que le HTTP), dans le meme appel que la resolution.
        log_review_sanction(
            &self.moderation_uc,
            &self.bot_config_repo,
            &self.broadcaster,
            &review,
            &req.applied_action,
            &req.resolved_by_id,
            &req.resolved_by_name,
        )
        .await;

        self.broadcast_resolution(
            "automod_review_resolved",
            &review,
            &req.applied_action,
            &req.resolved_by_id,
            &req.resolved_by_name,
        );
        Ok(Response::new(review_to_proto(review, false, None)))
    }

    async fn ignore_review(
        &self,
        request: Request<proto::IgnoreReviewRequest>,
    ) -> Result<Response<proto::AutomodReview>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.review_id)?;
        let review = self
            .reviews_uc
            .close_ignored(CloseIgnoredCommand {
                review_id: id,
                actor_id: req.actor_id.clone(),
                actor_name: req.actor_name.clone(),
                source: "discord".into(),
                requester: req.requester.map(facts_from_proto),
            })
            .await
            .map_err(domain_to_status)?;

        self.broadcast_resolution(
            "automod_review_resolved",
            &review,
            "ignore",
            &req.actor_id,
            &req.actor_name,
        );
        Ok(Response::new(review_to_proto(review, false, None)))
    }

    async fn reopen_review(
        &self,
        request: Request<proto::ReopenReviewRequest>,
    ) -> Result<Response<proto::AutomodReview>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.review_id)?;
        let deadline_hours = if req.deadline_hours > 0 {
            req.deadline_hours
        } else {
            72
        };
        let review = self
            .reviews_uc
            .reopen(ReopenReviewCommand {
                review_id: id,
                actor_id: req.actor_id.clone(),
                actor_name: req.actor_name.clone(),
                deadline_hours,
                source: "discord".into(),
                requester: req.requester.map(facts_from_proto),
            })
            .await
            .map_err(domain_to_status)?;

        self.broadcaster.broadcast(
            "automod_review_reopened",
            serde_json::json!({
                "review_id": review.id.to_string(),
                "action_id": review.id.to_string(),
                "guild_id": review.guild_id.as_str(),
                "user_id": review.user_id.as_str(),
                "actor": { "source": "discord", "id": &req.actor_id, "name": &req.actor_name },
            }),
        );
        Ok(Response::new(review_to_proto(review, false, None)))
    }

    async fn vote(
        &self,
        request: Request<proto::VoteRequest>,
    ) -> Result<Response<proto::ReviewVoteList>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.review_id)?;
        let requester = req.requester.map(facts_from_proto).unwrap_or_default();
        let votes = self
            .reviews_uc
            .cast_vote(CastVoteCommand {
                review_id: id,
                voter_id: req.voter_id.clone(),
                voter_name: req.voter_name.clone(),
                vote_action: req.vote_action.clone(),
                requester,
            })
            .await
            .map_err(domain_to_status)?;
        self.broadcaster.broadcast(
            "automod_review_voted",
            serde_json::json!({ "review_id": req.review_id, "votes": votes.len() }),
        );
        Ok(Response::new(votes_to_proto(votes)))
    }

    async fn list_votes(
        &self,
        request: Request<proto::ListVotesRequest>,
    ) -> Result<Response<proto::ReviewVoteList>, Status> {
        let id = parse_uuid(&request.into_inner().review_id)?;
        let votes = self
            .reviews_uc
            .list_votes(id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(votes_to_proto(votes)))
    }

    // ── Salons de discussion (tranche 2) ──

    async fn get_discussion(
        &self,
        request: Request<proto::GetDiscussionRequest>,
    ) -> Result<Response<proto::GetDiscussionResponse>, Status> {
        let id = parse_uuid(&request.into_inner().review_id)?;
        let discussion = self
            .reviews_uc
            .get_discussion(id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::GetDiscussionResponse {
            channel: discussion.map(|d| proto::DiscussionChannel {
                channel_id: d.channel_id,
                created: false,
            }),
        }))
    }

    async fn open_discussion(
        &self,
        request: Request<proto::OpenDiscussionRequest>,
    ) -> Result<Response<proto::DiscussionChannel>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.review_id)?;
        let (discussion, created) = self
            .reviews_uc
            .open_discussion(OpenDiscussionCommand {
                review_id: id,
                guild_id: req.guild_id.clone(),
                channel_id: req.channel_id,
                opened_by_id: req.opened_by_id.clone(),
                opened_by_name: req.opened_by_name,
                requester: req.requester.map(facts_from_proto).unwrap_or_default(),
            })
            .await
            .map_err(domain_to_status)?;

        if created {
            self.broadcaster.broadcast(
                "automod_discussion_opened",
                serde_json::json!({
                    "review_id": req.review_id,
                    "guild_id": req.guild_id,
                    "channel_id": discussion.channel_id,
                    "opened_by_id": req.opened_by_id,
                }),
            );
        }

        Ok(Response::new(proto::DiscussionChannel {
            channel_id: discussion.channel_id,
            created,
        }))
    }

    async fn delete_discussion(
        &self,
        request: Request<proto::DeleteDiscussionRequest>,
    ) -> Result<Response<proto::DeleteDiscussionResponse>, Status> {
        let id = parse_uuid(&request.into_inner().review_id)?;
        self.reviews_uc
            .delete_discussion(id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::DeleteDiscussionResponse {}))
    }

    async fn append_discussion_messages(
        &self,
        request: Request<proto::AppendDiscussionMessagesRequest>,
    ) -> Result<Response<proto::AppendDiscussionMessagesResponse>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.review_id)?;
        // Messages au timestamp illisible ignores (comme le handler HTTP).
        let messages: Vec<DiscussionMessage> = req
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
        let inserted = self
            .reviews_uc
            .append_discussion_messages(messages)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::AppendDiscussionMessagesResponse {
            inserted,
        }))
    }
}

impl AutomodReviewGrpc {
    /// Salon de discussion lie (best-effort ; ignore une erreur DB, comme le HTTP).
    async fn discussion_channel_id(&self, review_id: uuid::Uuid) -> Option<String> {
        match self.reviews_uc.get_discussion(review_id).await {
            Ok(Some(d)) => Some(d.channel_id),
            _ => None,
        }
    }

    /// Event WebSocket + Redis Stream commun a resolve/ignore (le bot listener
    /// edite la carte + applique l'action Discord en miroir des resolutions web).
    fn broadcast_resolution(
        &self,
        event: &str,
        review: &AutomodReview,
        applied_action: &str,
        actor_id: &str,
        actor_name: &str,
    ) {
        self.broadcaster.broadcast(
            event,
            serde_json::json!({
                "review_id": review.id.to_string(),
                "action_id": review.id.to_string(),
                "guild_id": review.guild_id.as_str(),
                "user_id": review.user_id.as_str(),
                "applied_action": applied_action,
                "actor": { "source": "discord", "id": actor_id, "name": actor_name },
            }),
        );
    }
}

fn parse_uuid(raw: &str) -> Result<uuid::Uuid, Status> {
    uuid::Uuid::parse_str(raw)
        .map_err(|_| Status::invalid_argument("review_id invalide (UUID attendu)"))
}

fn parse_json_or_empty_object(raw: &str) -> Result<serde_json::Value, Status> {
    if raw.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    serde_json::from_str(raw)
        .map_err(|e| Status::invalid_argument(format!("flags JSON invalide : {e}")))
}

fn facts_from_proto(f: proto::ModeratorFacts) -> ModeratorFacts {
    ModeratorFacts {
        is_admin: f.is_admin,
        has_moderate_members: f.has_moderate_members,
        has_manage_messages: f.has_manage_messages,
        has_mod_role: f.has_mod_role,
        has_admin_role: f.has_admin_role,
    }
}

fn votes_to_proto(votes: Vec<ReviewVote>) -> proto::ReviewVoteList {
    proto::ReviewVoteList {
        votes: votes
            .into_iter()
            .map(|v| proto::ReviewVote {
                voter_id: v.voter_id,
                voter_name: v.voter_name,
                vote_action: v.vote_action,
            })
            .collect(),
    }
}

fn json_to_string(v: &serde_json::Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string())
}

fn review_to_proto(
    r: AutomodReview,
    merged: bool,
    discussion_channel_id: Option<String>,
) -> proto::AutomodReview {
    proto::AutomodReview {
        id: r.id.to_string(),
        guild_id: r.guild_id.to_string(),
        channel_id: r.channel_id.to_string(),
        message_id: r.message_id.to_string(),
        user_id: r.user_id.to_string(),
        user_name: r.user_name,
        content_preview: r.content_preview,
        suggested_action: r.suggested_action,
        score: r.score,
        reason: r.reason,
        flags_json: json_to_string(&r.flags),
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
        incidents_json: json_to_string(&r.incidents),
        merged,
        discussion_channel_id,
    }
}

#[cfg(test)]
#[path = "tests/reviews.rs"]
mod tests;
