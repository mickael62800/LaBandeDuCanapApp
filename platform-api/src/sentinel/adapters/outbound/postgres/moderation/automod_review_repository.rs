use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::pg_err_ctx;
use platform_core::sentinel::domain::entities::moderation::review::automod::AutomodReview;
use platform_core::sentinel::domain::entities::moderation::review::automod::DiscussionChannel;
use platform_core::sentinel::domain::entities::moderation::review::automod::DiscussionMessage;
use platform_core::sentinel::domain::entities::moderation::review::automod::ExpiredReviewCard;
use platform_core::sentinel::domain::entities::moderation::review::automod::FpTerminalReview;
use platform_core::sentinel::domain::entities::moderation::review::automod::NewAutomodReview;
use platform_core::sentinel::domain::entities::moderation::review::automod::NewDiscussionChannel;
use platform_core::sentinel::domain::entities::moderation::review::automod::ReviewVote;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::moderation::automod_review_repository::AutomodReviewRepository;

const TBL: &str = "automod_reviews";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

/// Construit l'entree JSON d'un incident (pour la liste agregee `incidents`).
fn incident_json(r: &NewAutomodReview) -> serde_json::Value {
    serde_json::json!({
        "message_id": r.message_id.as_str(),
        "channel_id": r.channel_id.as_str(),
        "content_preview": r.content_preview,
        "score": r.score,
        "reason": r.reason,
        "suggested_action": r.suggested_action.as_str(),
        "at": chrono::Utc::now().to_rfc3339(),
    })
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    guild_id: String,
    channel_id: String,
    message_id: String,
    user_id: String,
    user_name: String,
    content_preview: String,
    suggested_action: String,
    score: f64,
    reason: String,
    flags: serde_json::Value,
    status: String,
    applied_action: Option<String>,
    resolved_by_id: Option<String>,
    resolved_by_name: Option<String>,
    resolved_source: Option<String>,
    created_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
    voting_deadline: Option<DateTime<Utc>>,
    decided_action: Option<String>,
    #[sqlx(default)]
    quorum_met: bool,
    decided_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    incident_count: i32,
    #[sqlx(default)]
    cumulative_score: f64,
    #[sqlx(default)]
    incidents: serde_json::Value,
    #[sqlx(default)]
    sanction_logged: bool,
}

impl From<Row> for AutomodReview {
    fn from(r: Row) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            channel_id: r.channel_id.into(),
            message_id: r.message_id.into(),
            user_id: r.user_id.into(),
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
            created_at: r.created_at,
            resolved_at: r.resolved_at,
            voting_deadline: r.voting_deadline,
            decided_action: r.decided_action,
            quorum_met: r.quorum_met,
            decided_at: r.decided_at,
            incident_count: r.incident_count,
            cumulative_score: r.cumulative_score,
            incidents: if r.incidents.is_null() {
                serde_json::json!([])
            } else {
                r.incidents
            },
            sanction_logged: r.sanction_logged,
        }
    }
}

#[derive(sqlx::FromRow)]
struct VoteRow {
    id: Uuid,
    review_id: Uuid,
    voter_id: String,
    voter_name: String,
    vote_action: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct DiscussionRow {
    id: Uuid,
    review_id: Uuid,
    guild_id: String,
    channel_id: String,
    opened_by_id: String,
    opened_by_name: String,
    created_at: DateTime<Utc>,
}

impl From<DiscussionRow> for DiscussionChannel {
    fn from(r: DiscussionRow) -> Self {
        Self {
            id: r.id,
            review_id: r.review_id,
            guild_id: r.guild_id,
            channel_id: r.channel_id,
            opened_by_id: r.opened_by_id,
            opened_by_name: r.opened_by_name,
            created_at: r.created_at,
        }
    }
}

impl From<VoteRow> for ReviewVote {
    fn from(r: VoteRow) -> Self {
        Self {
            id: r.id,
            review_id: r.review_id,
            voter_id: r.voter_id,
            voter_name: r.voter_name,
            vote_action: r.vote_action,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct PgAutomodReviewRepository {
    pool: PgPool,
}

/// Palier de suggestion d'une carte agregee. Les incidents d'une meme carte
/// sont une escalation : conserver uniquement l'action du dernier message
/// laisserait une carte a "warn" alors que son score cumule a deja atteint le
/// seuil de mute. Ces valeurs correspondent aux seuils baseline exposes dans
/// l'interface (warn=2, delete=4, mute=6, ban=9).
fn action_for_cumulative_score(score: f64) -> &'static str {
    if score >= 9.0 {
        "ban"
    } else if score >= 6.0 {
        "mute"
    } else if score >= 4.0 {
        "delete"
    } else {
        "warn"
    }
}

impl PgAutomodReviewRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

mod discussions;
mod lifecycle;
mod reviews;
mod votes;

#[async_trait]
impl AutomodReviewRepository for PgAutomodReviewRepository {
    async fn create(&self, r: NewAutomodReview) -> Result<AutomodReview, DomainError> {
        self.create_impl(r).await
    }

    async fn create_or_merge(
        &self,
        r: NewAutomodReview,
        aggregate: bool,
        window_minutes: i64,
    ) -> Result<(AutomodReview, bool), DomainError> {
        self.create_or_merge_impl(r, aggregate, window_minutes)
            .await
    }

    async fn get(&self, id: Uuid) -> Result<Option<AutomodReview>, DomainError> {
        self.get_impl(id).await
    }

    async fn find_by_message_id(
        &self,
        guild_id: &str,
        message_id: &str,
    ) -> Result<Option<AutomodReview>, DomainError> {
        self.find_by_message_id_impl(guild_id, message_id).await
    }

    async fn list_pending(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError> {
        self.list_pending_impl(guild_id, limit).await
    }

    async fn list_recent(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError> {
        self.list_recent_impl(guild_id, limit).await
    }

    async fn resolve(
        &self,
        id: Uuid,
        applied_action: &str,
        resolved_by_id: &str,
        resolved_by_name: &str,
        resolved_source: &str,
    ) -> Result<AutomodReview, DomainError> {
        self.resolve_impl(
            id,
            applied_action,
            resolved_by_id,
            resolved_by_name,
            resolved_source,
        )
        .await
    }

    async fn close_ignored(
        &self,
        id: Uuid,
        actor_id: &str,
        actor_name: &str,
        source: &str,
    ) -> Result<AutomodReview, DomainError> {
        self.close_ignored_impl(id, actor_id, actor_name, source)
            .await
    }

    async fn reopen(&self, id: Uuid, deadline_hours: i64) -> Result<AutomodReview, DomainError> {
        self.reopen_impl(id, deadline_hours).await
    }

    async fn upsert_vote(
        &self,
        review_id: Uuid,
        voter_id: &str,
        voter_name: &str,
        vote_action: &str,
    ) -> Result<(), DomainError> {
        self.upsert_vote_impl(review_id, voter_id, voter_name, vote_action)
            .await
    }

    async fn list_votes(&self, review_id: Uuid) -> Result<Vec<ReviewVote>, DomainError> {
        self.list_votes_impl(review_id).await
    }

    async fn decide(
        &self,
        id: Uuid,
        decided_action: &str,
        quorum_met: bool,
    ) -> Result<AutomodReview, DomainError> {
        self.decide_impl(id, decided_action, quorum_met).await
    }

    async fn list_expired_voting(&self, limit: i64) -> Result<Vec<AutomodReview>, DomainError> {
        self.list_expired_voting_impl(limit).await
    }

    async fn fp_terminal_reviews(
        &self,
        guild_id: &str,
        days: i64,
        limit: i64,
    ) -> Result<Vec<FpTerminalReview>, DomainError> {
        self.fp_terminal_reviews_impl(guild_id, days, limit).await
    }

    async fn find_discussion(
        &self,
        review_id: Uuid,
    ) -> Result<Option<DiscussionChannel>, DomainError> {
        self.find_discussion_impl(review_id).await
    }

    async fn create_discussion(
        &self,
        d: NewDiscussionChannel,
    ) -> Result<(DiscussionChannel, bool), DomainError> {
        self.create_discussion_impl(d).await
    }

    async fn delete_discussion(&self, review_id: Uuid) -> Result<(), DomainError> {
        self.delete_discussion_impl(review_id).await
    }

    async fn append_discussion_messages(
        &self,
        messages: &[DiscussionMessage],
    ) -> Result<u64, DomainError> {
        self.append_discussion_messages_impl(messages).await
    }

    async fn list_discussion_messages(
        &self,
        review_id: Uuid,
    ) -> Result<Vec<DiscussionMessage>, DomainError> {
        self.list_discussion_messages_impl(review_id).await
    }

    async fn expire_stale_decided(
        &self,
        grace_hours: i64,
        limit: i64,
    ) -> Result<Vec<ExpiredReviewCard>, DomainError> {
        self.expire_stale_decided_impl(grace_hours, limit).await
    }

    async fn expire_review_cards(
        &self,
        days: i64,
        limit: i64,
    ) -> Result<Vec<ExpiredReviewCard>, DomainError> {
        self.expire_review_cards_impl(days, limit).await
    }
}

#[derive(sqlx::FromRow)]
struct DiscussionMsgRow {
    review_id: Uuid,
    discord_message_id: String,
    author_id: String,
    author_name: String,
    author_is_bot: bool,
    content: String,
    sent_at: DateTime<Utc>,
}

impl From<DiscussionMsgRow> for DiscussionMessage {
    fn from(r: DiscussionMsgRow) -> Self {
        DiscussionMessage {
            review_id: r.review_id,
            discord_message_id: r.discord_message_id,
            author_id: r.author_id,
            author_name: r.author_name,
            author_is_bot: r.author_is_bot,
            content: r.content,
            sent_at: r.sent_at,
        }
    }
}
