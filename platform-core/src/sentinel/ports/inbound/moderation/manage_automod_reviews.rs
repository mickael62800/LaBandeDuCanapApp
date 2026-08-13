//! Use case Automod review (cards moderation).
//!
//! Le HTTP handler appelle ce port — jamais directement le repo. Permet
//! d'isoler la regle metier (idempotence, vote, depouillement) du transport.

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::moderation::review::automod::AutomodReview;
use crate::sentinel::domain::entities::moderation::review::automod::DiscussionChannel;
use crate::sentinel::domain::entities::moderation::review::automod::DiscussionMessage;
use crate::sentinel::domain::entities::moderation::review::automod::ExpiredReviewCard;
use crate::sentinel::domain::entities::moderation::review::automod::FpStats;
use crate::sentinel::domain::entities::moderation::review::automod::ModeratorFacts;
use crate::sentinel::domain::entities::moderation::review::automod::NewAutomodReview;
use crate::sentinel::domain::entities::moderation::review::automod::ReviewVote;
use crate::sentinel::domain::entities::moderation::review::automod::TallyResult;
use crate::sentinel::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct ResolveAutomodReviewCommand {
    pub review_id: Uuid,
    /// Action choisie : "warn", "delete", "mute", "ban", "ignore".
    pub applied_action: String,
    pub resolved_by_id: String,
    pub resolved_by_name: String,
    /// "discord" ou "web".
    pub resolved_source: String,
    /// Faits du demandeur pour appliquer la regle `can_finalize_review`
    /// (source "discord"). `None` pour la source "web" (autorisee par le
    /// middleware guild_auth en amont).
    pub requester: Option<ModeratorFacts>,
}

/// Clore immediatement une review en "ignore" (sans attendre le vote/la
/// finalisation). Ouvert a tout moderateur (regle `is_moderator`).
#[derive(Debug, Clone)]
pub struct CloseIgnoredCommand {
    pub review_id: Uuid,
    pub actor_id: String,
    pub actor_name: String,
    /// "discord" ou "web".
    pub source: String,
    /// Faits du demandeur (source "discord"). `None` pour "web".
    pub requester: Option<ModeratorFacts>,
}

/// Rouvrir un dossier resolu/ignore : repasse en 'voting' avec une nouvelle
/// echeance. Ouvert a tout moderateur (regle `is_moderator`).
#[derive(Debug, Clone)]
pub struct ReopenReviewCommand {
    pub review_id: Uuid,
    pub actor_id: String,
    pub actor_name: String,
    /// Duree (heures) de la nouvelle fenetre de vote.
    pub deadline_hours: i64,
    /// "discord" ou "web".
    pub source: String,
    /// Faits du demandeur (source "discord"). `None` pour "web".
    pub requester: Option<ModeratorFacts>,
}

/// Vote d'un moderateur sur une review en cours.
#[derive(Debug, Clone)]
pub struct CastVoteCommand {
    pub review_id: Uuid,
    pub voter_id: String,
    pub voter_name: String,
    /// "warn" | "delete" | "mute" | "ban" | "ignore".
    pub vote_action: String,
    /// Faits du demandeur pour appliquer la regle `is_moderator`.
    pub requester: ModeratorFacts,
}

#[async_trait]
pub trait ManageAutomodReviewsUseCase: Send + Sync {
    async fn create(&self, review: NewAutomodReview) -> Result<AutomodReview, DomainError>;

    /// Cree ou agrege une review (cf. `AutomodReviewRepository::create_or_merge`).
    /// Retourne `(review, merged)`.
    async fn create_or_merge(
        &self,
        review: NewAutomodReview,
        aggregate: bool,
        // Fenetre d'inactivite (minutes) au-dela de laquelle on n'agrege plus
        // dans une carte existante (0 = pas de limite).
        window_minutes: i64,
    ) -> Result<(AutomodReview, bool), DomainError>;
    async fn get(&self, id: Uuid) -> Result<Option<AutomodReview>, DomainError>;
    /// Retrouve la review la plus recente associee a un message Discord.
    async fn find_by_message_id(
        &self,
        guild_id: &str,
        message_id: &str,
    ) -> Result<Option<AutomodReview>, DomainError>;
    async fn list_pending(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError>;
    async fn list_recent(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError>;
    async fn resolve(&self, cmd: ResolveAutomodReviewCommand)
        -> Result<AutomodReview, DomainError>;

    /// Clore immediatement en "ignore" (statut pending|voting|decided ->
    /// ignored). Reserve aux moderateurs (`is_moderator`).
    async fn close_ignored(&self, cmd: CloseIgnoredCommand) -> Result<AutomodReview, DomainError>;

    /// Rouvrir un dossier (applied|ignored -> voting), reinitialise les votes
    /// et fixe une nouvelle echeance. Reserve aux moderateurs (`is_moderator`).
    async fn reopen(&self, cmd: ReopenReviewCommand) -> Result<AutomodReview, DomainError>;

    // ── Vote ──
    /// Enregistre/met a jour un vote, retourne la liste des votes a jour.
    async fn cast_vote(&self, cmd: CastVoteCommand) -> Result<Vec<ReviewVote>, DomainError>;

    /// Liste les votes d'une review.
    async fn list_votes(&self, review_id: Uuid) -> Result<Vec<ReviewVote>, DomainError>;

    /// Cloture le vote : depouille (quorum + tie-break) et passe en
    /// 'decided'. Retourne la review et le resultat du depouillement.
    async fn decide(
        &self,
        review_id: Uuid,
        quorum: usize,
        tie_action: &str,
    ) -> Result<(AutomodReview, TallyResult), DomainError>;

    /// Reviews en vote dont l'echeance est depassee (job worker).
    async fn list_expired_voting(&self, limit: i64) -> Result<Vec<AutomodReview>, DomainError>;

    /// Cartes de review closes (applied|ignored) resolues depuis plus de
    /// `days` jours et encore mappees a un message Discord. Retire le mapping
    /// (pour ne pas re-traiter) et retourne la liste a faire expirer cote bot.
    /// Expire les reviews 'decided' jamais finalisees (verdict lapse) et renvoie
    /// leurs cartes a supprimer.
    async fn expire_stale_decided_reviews(
        &self,
        grace_hours: i64,
        limit: i64,
    ) -> Result<Vec<ExpiredReviewCard>, DomainError>;

    async fn expired_review_cards(
        &self,
        days: i64,
        limit: i64,
    ) -> Result<Vec<ExpiredReviewCard>, DomainError>;

    /// Mesure les faux positifs (over-block) de l'automod sur la fenetre
    /// glissante `days` (clampe 1..=365) : taux global, par flag detecteur et
    /// par action suggeree. Lecture seule, agregation locale.
    async fn fp_stats(&self, guild_id: &str, days: i64) -> Result<FpStats, DomainError>;

    // ── Salon de discussion ──
    /// Salon de discussion deja ouvert pour cette review, le cas echeant.
    async fn get_discussion(
        &self,
        review_id: Uuid,
    ) -> Result<Option<DiscussionChannel>, DomainError>;

    /// Ouvre (enregistre) un salon de discussion : applique la regle d'acces
    /// (`can_open_discussion`) puis persiste de facon idempotente. Retourne
    /// `(salon, created)`. `Forbidden` si le demandeur n'est pas autorise.
    async fn open_discussion(
        &self,
        cmd: OpenDiscussionCommand,
    ) -> Result<(DiscussionChannel, bool), DomainError>;

    /// Supprime l'enregistrement du salon de discussion (le salon Discord a
    /// ete supprime a la main) pour permettre d'en rouvrir un neuf.
    async fn delete_discussion(&self, review_id: Uuid) -> Result<(), DomainError>;

    /// Persiste le transcript d'un salon de discussion (batch idempotent).
    async fn append_discussion_messages(
        &self,
        messages: Vec<DiscussionMessage>,
    ) -> Result<u64, DomainError>;

    /// Liste le transcript d'une review (ordre chronologique).
    async fn list_discussion_messages(
        &self,
        review_id: Uuid,
    ) -> Result<Vec<DiscussionMessage>, DomainError>;
}

/// Commande d'ouverture d'un salon de discussion. Les `requester` sont les
/// faits Discord fournis par l'adapter bot ; la DECISION est prise ici.
#[derive(Debug, Clone)]
pub struct OpenDiscussionCommand {
    pub review_id: Uuid,
    pub guild_id: String,
    pub channel_id: String,
    pub opened_by_id: String,
    pub opened_by_name: String,
    pub requester: ModeratorFacts,
}
