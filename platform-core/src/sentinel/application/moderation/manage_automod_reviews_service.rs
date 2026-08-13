//! Service application Automod reviews.
//!
//! Regles :
//!   * `applied_action` doit etre une valeur reconnue ('warn'|'delete'|'mute'|'ban'|'ignore').
//!   * `resolved_source` doit etre 'web' ou 'discord'.
//!   * idempotence : la 2e resolve sur la meme review renvoie `Conflict`
//!     (le repo Postgres garantit ca via `UPDATE WHERE status='pending'`).

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::moderation::review::automod::tally_votes;
use crate::sentinel::domain::entities::moderation::review::automod::AppliedAction;
use crate::sentinel::domain::entities::moderation::review::automod::AutomodReview;
use crate::sentinel::domain::entities::moderation::review::automod::ExpiredReviewCard;
use crate::sentinel::domain::entities::moderation::review::automod::ModeratorFacts;
use crate::sentinel::domain::entities::moderation::review::automod::NewAutomodReview;
use crate::sentinel::domain::entities::moderation::review::automod::ReviewVote;
use crate::sentinel::domain::entities::moderation::review::automod::TallyResult;
use crate::sentinel::domain::entities::moderation::review::automod::TieAction;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::manage_automod_reviews::CastVoteCommand;
use crate::sentinel::ports::inbound::moderation::manage_automod_reviews::CloseIgnoredCommand;
use crate::sentinel::ports::inbound::moderation::manage_automod_reviews::ManageAutomodReviewsUseCase;
use crate::sentinel::ports::inbound::moderation::manage_automod_reviews::ReopenReviewCommand;
use crate::sentinel::ports::inbound::moderation::manage_automod_reviews::ResolveAutomodReviewCommand;
use crate::sentinel::ports::outbound::moderation::automod_review_repository::AutomodReviewRepository;

pub struct ManageAutomodReviewsService {
    repo: Arc<dyn AutomodReviewRepository>,
}

impl ManageAutomodReviewsService {
    pub fn new(repo: Arc<dyn AutomodReviewRepository>) -> Self {
        Self { repo }
    }

    /// Conflit d'interet : refuse qu'un acteur agisse (vote / finalisation /
    /// cloture) sur une detection qui le vise lui-meme (`actor_id` ==
    /// `review.user_id`). No-op si la review est introuvable (le repo renverra
    /// l'erreur appropriee plus loin).
    ///
    /// EXCEPTION — pleins pouvoirs : un administrateur / fondateur (au sens de
    /// `can_finalize_review`) peut passer outre. Le garde protege le conflit
    /// d'interet entre moderateurs ordinaires ; il ne doit pas menotter
    /// l'autorite qui, precisement, tranche en dernier ressort. Cote web, tout
    /// appelant a franchi `SUPERADMIN_USER_IDS` -> facts Owner -> bypass.
    async fn reject_self_action(
        &self,
        review_id: Uuid,
        actor_id: &str,
        verb: &str,
        requester: Option<&ModeratorFacts>,
    ) -> Result<(), DomainError> {
        let privilegie = requester
            .map(
                crate::sentinel::domain::entities::moderation::review::automod::can_finalize_review,
            )
            .unwrap_or(false);
        if privilegie {
            return Ok(());
        }
        if let Some(review) = self.repo.get(review_id).await? {
            if review.user_id.as_str() == actor_id {
                return Err(DomainError::ValidationError(format!(
                    "Tu ne peux pas {verb} ta propre detection."
                )));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ManageAutomodReviewsUseCase for ManageAutomodReviewsService {
    async fn create(&self, review: NewAutomodReview) -> Result<AutomodReview, DomainError> {
        crate::sentinel::application::validation::validate_guild_id(&review.guild_id)?;
        self.repo.create(review).await
    }

    async fn create_or_merge(
        &self,
        review: NewAutomodReview,
        aggregate: bool,
        window_minutes: i64,
    ) -> Result<(AutomodReview, bool), DomainError> {
        crate::sentinel::application::validation::validate_guild_id(&review.guild_id)?;
        self.repo
            .create_or_merge(review, aggregate, window_minutes)
            .await
    }

    async fn get(&self, id: Uuid) -> Result<Option<AutomodReview>, DomainError> {
        self.repo.get(id).await
    }

    async fn find_by_message_id(
        &self,
        guild_id: &str,
        message_id: &str,
    ) -> Result<Option<AutomodReview>, DomainError> {
        if guild_id.trim().is_empty() || message_id.trim().is_empty() {
            return Ok(None);
        }
        self.repo.find_by_message_id(guild_id, message_id).await
    }

    async fn list_pending(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError> {
        crate::sentinel::application::validation::validate_guild_id(guild_id)?;
        self.repo
            .list_pending(
                guild_id,
                limit.clamp(1, crate::sentinel::application::validation::PAGE_LIMIT_MAX),
            )
            .await
    }

    async fn list_recent(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError> {
        crate::sentinel::application::validation::validate_guild_id(guild_id)?;
        self.repo
            .list_recent(
                guild_id,
                limit.clamp(1, crate::sentinel::application::validation::PAGE_LIMIT_MAX),
            )
            .await
    }

    async fn resolve(
        &self,
        cmd: ResolveAutomodReviewCommand,
    ) -> Result<AutomodReview, DomainError> {
        if AppliedAction::from_str(&cmd.applied_action).is_none() {
            return Err(DomainError::ValidationError(format!(
                "applied_action invalide : {}. Valeurs : warn|delete|mute|ban|ignore",
                cmd.applied_action
            )));
        }
        if !matches!(cmd.resolved_source.as_str(), "web" | "discord") {
            return Err(DomainError::ValidationError(
                "resolved_source doit etre 'web' ou 'discord'".into(),
            ));
        }
        crate::sentinel::application::validation::validate_non_empty(
            &cmd.resolved_by_id,
            "resolved_by_id",
        )?;
        // Conflit d'interet : l'utilisateur flagge ne peut pas finaliser sa
        // propre detection.
        self.reject_self_action(
            cmd.review_id,
            &cmd.resolved_by_id,
            "finaliser",
            cmd.requester.as_ref(),
        )
        .await?;
        // Regle d'acces (domaine) : finalisation Discord reservee aux admins.
        // La source "web" est autorisee en amont par le middleware guild_auth.
        if let Some(facts) = &cmd.requester {
            if !crate::sentinel::domain::entities::moderation::review::automod::can_finalize_review(
                facts,
            ) {
                return Err(DomainError::Forbidden(
                    "Seul un administrateur peut finaliser.".into(),
                ));
            }
        }
        self.repo
            .resolve(
                cmd.review_id,
                &cmd.applied_action,
                &cmd.resolved_by_id,
                &cmd.resolved_by_name,
                &cmd.resolved_source,
            )
            .await
    }

    async fn close_ignored(&self, cmd: CloseIgnoredCommand) -> Result<AutomodReview, DomainError> {
        if !matches!(cmd.source.as_str(), "web" | "discord") {
            return Err(DomainError::ValidationError(
                "source doit etre 'web' ou 'discord'".into(),
            ));
        }
        crate::sentinel::application::validation::validate_non_empty(&cmd.actor_id, "actor_id")?;
        // Conflit d'interet : l'utilisateur flagge ne peut pas clore sa propre
        // detection.
        self.reject_self_action(
            cmd.review_id,
            &cmd.actor_id,
            "clore",
            cmd.requester.as_ref(),
        )
        .await?;
        // Regle d'acces (domaine) : tout moderateur peut clore (source discord).
        // La source "web" est autorisee en amont par le middleware guild_auth.
        if let Some(facts) = &cmd.requester {
            if !crate::sentinel::domain::entities::moderation::review::automod::is_moderator(facts)
            {
                return Err(DomainError::Forbidden(
                    "Seul un moderateur peut clore ce dossier.".into(),
                ));
            }
        }
        self.repo
            .close_ignored(cmd.review_id, &cmd.actor_id, &cmd.actor_name, &cmd.source)
            .await
    }

    async fn reopen(&self, cmd: ReopenReviewCommand) -> Result<AutomodReview, DomainError> {
        crate::sentinel::application::validation::validate_non_empty(&cmd.actor_id, "actor_id")?;
        if let Some(facts) = &cmd.requester {
            if !crate::sentinel::domain::entities::moderation::review::automod::is_moderator(facts)
            {
                return Err(DomainError::Forbidden(
                    "Seul un moderateur peut rouvrir ce dossier.".into(),
                ));
            }
        }
        let hours = crate::sentinel::domain::entities::moderation::review::automod::clamp_vote_deadline_hours(
            cmd.deadline_hours,
        );
        self.repo.reopen(cmd.review_id, hours).await
    }

    async fn cast_vote(&self, cmd: CastVoteCommand) -> Result<Vec<ReviewVote>, DomainError> {
        // Regle d'acces (domaine) : seul un moderateur peut voter.
        if !crate::sentinel::domain::entities::moderation::review::automod::is_moderator(
            &cmd.requester,
        ) {
            return Err(DomainError::Forbidden(
                "Tu n'es pas autorise a voter.".into(),
            ));
        }
        if AppliedAction::from_str(&cmd.vote_action).is_none() {
            return Err(DomainError::ValidationError(format!(
                "vote_action invalide : {}. Valeurs : warn|delete|mute|ban|ignore",
                cmd.vote_action
            )));
        }
        crate::sentinel::application::validation::validate_non_empty(&cmd.voter_id, "voter_id")?;
        // Conflit d'interet : l'utilisateur flagge ne peut pas voter sur sa
        // propre detection.
        self.reject_self_action(
            cmd.review_id,
            &cmd.voter_id,
            "voter sur",
            Some(&cmd.requester),
        )
        .await?;
        self.repo
            .upsert_vote(
                cmd.review_id,
                &cmd.voter_id,
                &cmd.voter_name,
                &cmd.vote_action,
            )
            .await?;
        self.repo.list_votes(cmd.review_id).await
    }

    async fn list_votes(&self, review_id: Uuid) -> Result<Vec<ReviewVote>, DomainError> {
        self.repo.list_votes(review_id).await
    }

    async fn decide(
        &self,
        review_id: Uuid,
        quorum: usize,
        tie_action: &str,
    ) -> Result<(AutomodReview, TallyResult), DomainError> {
        // Borne saine du quorum : règle du dépouillement, pas du handler.
        let quorum = quorum.clamp(1, 100);
        let votes = self.repo.list_votes(review_id).await?;
        let actions: Vec<AppliedAction> = votes
            .iter()
            .filter_map(|v| AppliedAction::from_str(&v.vote_action))
            .collect();
        let tally = tally_votes(&actions, quorum, TieAction::from_str(tie_action));
        let review = self
            .repo
            .decide(review_id, tally.decided.as_str(), tally.quorum_met)
            .await?;
        Ok((review, tally))
    }

    async fn list_expired_voting(&self, limit: i64) -> Result<Vec<AutomodReview>, DomainError> {
        self.repo
            .list_expired_voting(
                limit.clamp(1, crate::sentinel::application::validation::PAGE_LIMIT_MAX),
            )
            .await
    }

    async fn expire_stale_decided_reviews(
        &self,
        grace_hours: i64,
        limit: i64,
    ) -> Result<Vec<ExpiredReviewCard>, DomainError> {
        self.repo
            .expire_stale_decided(
                grace_hours.clamp(1, 8760),
                limit.clamp(1, crate::sentinel::application::validation::BATCH_LIMIT_MAX),
            )
            .await
    }

    async fn expired_review_cards(
        &self,
        days: i64,
        limit: i64,
    ) -> Result<Vec<ExpiredReviewCard>, DomainError> {
        self.repo
            .expire_review_cards(
                days.clamp(1, 3650),
                limit.clamp(1, crate::sentinel::application::validation::BATCH_LIMIT_MAX),
            )
            .await
    }

    async fn fp_stats(
        &self,
        guild_id: &str,
        days: i64,
    ) -> Result<crate::sentinel::domain::entities::moderation::review::automod::FpStats, DomainError>
    {
        use crate::sentinel::domain::entities::moderation::review::automod::{
            compute_fp_stats, FP_STATS_MAX_ROWS,
        };
        crate::sentinel::application::validation::validate_guild_id(guild_id)?;
        let days = days.clamp(1, 365);
        let rows = self
            .repo
            .fp_terminal_reviews(guild_id, days, FP_STATS_MAX_ROWS)
            .await?;
        let capped = rows.len() as i64 >= FP_STATS_MAX_ROWS;
        if capped {
            tracing::warn!(
                guild_id,
                days,
                max = FP_STATS_MAX_ROWS,
                "fp-stats : echantillon tronque, stats approximatives"
            );
        }
        Ok(compute_fp_stats(days, &rows, capped))
    }

    async fn get_discussion(
        &self,
        review_id: Uuid,
    ) -> Result<
        Option<crate::sentinel::domain::entities::moderation::review::automod::DiscussionChannel>,
        DomainError,
    > {
        self.repo.find_discussion(review_id).await
    }

    async fn open_discussion(
        &self,
        cmd: crate::sentinel::ports::inbound::moderation::manage_automod_reviews::OpenDiscussionCommand,
    ) -> Result<
        (
            crate::sentinel::domain::entities::moderation::review::automod::DiscussionChannel,
            bool,
        ),
        DomainError,
    > {
        use crate::sentinel::domain::entities::moderation::review::automod::{
            can_open_discussion, NewDiscussionChannel,
        };

        // Regle d'acces (domaine) : le demandeur doit etre moderateur.
        if !can_open_discussion(&cmd.requester) {
            return Err(DomainError::Forbidden(
                "Tu n'es pas autorise a ouvrir une discussion.".into(),
            ));
        }
        crate::sentinel::application::validation::validate_non_empty(
            &cmd.channel_id,
            "channel_id",
        )?;
        // Pas de discussion sur une affaire deja close (sanction appliquee ou ignoree).
        if let Some(review) = self.repo.get(cmd.review_id).await? {
            if matches!(review.status.as_str(), "applied" | "ignored") {
                return Err(DomainError::Conflict(
                    "Cette review est close : impossible d'ouvrir une discussion.".into(),
                ));
            }
        } else {
            return Err(DomainError::NotFound(format!(
                "review {} introuvable",
                cmd.review_id
            )));
        }
        self.repo
            .create_discussion(NewDiscussionChannel {
                review_id: cmd.review_id,
                guild_id: cmd.guild_id,
                channel_id: cmd.channel_id,
                opened_by_id: cmd.opened_by_id,
                opened_by_name: cmd.opened_by_name,
            })
            .await
    }

    async fn delete_discussion(&self, review_id: Uuid) -> Result<(), DomainError> {
        self.repo.delete_discussion(review_id).await
    }

    async fn append_discussion_messages(
        &self,
        messages: Vec<
            crate::sentinel::domain::entities::moderation::review::automod::DiscussionMessage,
        >,
    ) -> Result<u64, DomainError> {
        if messages.is_empty() {
            return Ok(0);
        }
        self.repo.append_discussion_messages(&messages).await
    }

    async fn list_discussion_messages(
        &self,
        review_id: Uuid,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::moderation::review::automod::DiscussionMessage>,
        DomainError,
    > {
        self.repo.list_discussion_messages(review_id).await
    }
}

#[cfg(test)]
#[path = "tests/manage_automod_reviews.rs"]
mod tests;
