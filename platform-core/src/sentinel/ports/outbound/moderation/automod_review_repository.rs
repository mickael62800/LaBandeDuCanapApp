use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::moderation::review::automod::AutomodReview;
use crate::sentinel::domain::entities::moderation::review::automod::DiscussionChannel;
use crate::sentinel::domain::entities::moderation::review::automod::DiscussionMessage;
use crate::sentinel::domain::entities::moderation::review::automod::ExpiredReviewCard;
use crate::sentinel::domain::entities::moderation::review::automod::FpTerminalReview;
use crate::sentinel::domain::entities::moderation::review::automod::NewAutomodReview;
use crate::sentinel::domain::entities::moderation::review::automod::NewDiscussionChannel;
use crate::sentinel::domain::entities::moderation::review::automod::ReviewVote;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait AutomodReviewRepository: Send + Sync {
    async fn create(&self, review: NewAutomodReview) -> Result<AutomodReview, DomainError>;

    /// Cree une review, ou — si `aggregate` et qu'une carte 'voting' existe
    /// deja pour le meme (guild, user) — y agrege l'incident (liste, compteur,
    /// score cumule, score max, action la plus severe, deadline prolongee).
    /// Retourne `(review, merged)` : `merged = true` si l'incident a ete
    /// fusionne dans une carte existante.
    async fn create_or_merge(
        &self,
        review: NewAutomodReview,
        aggregate: bool,
        // Fenetre d'inactivite (minutes) ; 0 = pas de limite.
        window_minutes: i64,
    ) -> Result<(AutomodReview, bool), DomainError>;
    async fn get(&self, id: Uuid) -> Result<Option<AutomodReview>, DomainError>;
    /// Retrouve la review la plus recente associee a un message Discord
    /// (guild + message_id). Utile pour retrouver le review_id depuis une
    /// carte 1-clic (dont les boutons ne portent pas l'id).
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
    /// Resolve une review (statut pending OU decided -> applied|ignored).
    /// Retourne la review mise a jour ou `Conflict` si deja resolue.
    async fn resolve(
        &self,
        id: Uuid,
        applied_action: &str,
        resolved_by_id: &str,
        resolved_by_name: &str,
        resolved_source: &str,
    ) -> Result<AutomodReview, DomainError>;

    /// Clore immediatement en "ignore" (statut pending|voting|decided ->
    /// ignored). `Conflict` si la review est deja close (applied|ignored).
    async fn close_ignored(
        &self,
        id: Uuid,
        actor_id: &str,
        actor_name: &str,
        source: &str,
    ) -> Result<AutomodReview, DomainError>;

    /// Rouvrir un dossier (applied|ignored -> voting) : efface les votes,
    /// remet les champs de resolution a NULL et fixe une nouvelle echeance
    /// (NOW + `deadline_hours`). `Conflict` si la review n'est pas close.
    async fn reopen(&self, id: Uuid, deadline_hours: i64) -> Result<AutomodReview, DomainError>;

    // ── Vote ──
    /// Enregistre/met a jour le vote d'un moderateur (un seul par review et
    /// par votant). `Conflict` si la review n'est plus en statut 'voting'.
    async fn upsert_vote(
        &self,
        review_id: Uuid,
        voter_id: &str,
        voter_name: &str,
        vote_action: &str,
    ) -> Result<(), DomainError>;

    /// Liste les votes d'une review.
    async fn list_votes(&self, review_id: Uuid) -> Result<Vec<ReviewVote>, DomainError>;

    /// Passe une review de 'voting' a 'decided' avec le verdict calcule.
    /// `Conflict` si la review n'est plus en 'voting'.
    async fn decide(
        &self,
        id: Uuid,
        decided_action: &str,
        quorum_met: bool,
    ) -> Result<AutomodReview, DomainError>;

    /// Reviews en statut 'voting' dont l'echeance est depassee (job worker).
    async fn list_expired_voting(&self, limit: i64) -> Result<Vec<AutomodReview>, DomainError>;

    /// Cartes closes (applied|ignored) resolues depuis plus de `days` jours et
    /// encore mappees a un message Discord. Retire le mapping `automod_review`
    /// de `discord_action_messages` pour les cartes retournees.
    async fn expire_review_cards(
        &self,
        days: i64,
        limit: i64,
    ) -> Result<Vec<ExpiredReviewCard>, DomainError>;

    /// Reviews restees en 'decided' (verdict de vote calcule mais jamais
    /// finalise par un admin) depuis plus de `grace_hours` : on les passe en
    /// 'ignored' (le verdict lapse) et on renvoie leurs cartes a nettoyer.
    async fn expire_stale_decided(
        &self,
        grace_hours: i64,
        limit: i64,
    ) -> Result<Vec<ExpiredReviewCard>, DomainError>;

    /// Charge les reviews terminales (statut applied|ignored|decided) de la
    /// fenetre glissante `days`, bornees a `limit`, pour l'agregation des faux
    /// positifs (over-block). Ordre : plus recentes d'abord.
    async fn fp_terminal_reviews(
        &self,
        guild_id: &str,
        days: i64,
        limit: i64,
    ) -> Result<Vec<FpTerminalReview>, DomainError>;

    // ── Salon de discussion ──
    /// Salon de discussion deja ouvert pour cette review, le cas echeant.
    async fn find_discussion(
        &self,
        review_id: Uuid,
    ) -> Result<Option<DiscussionChannel>, DomainError>;

    /// Enregistre un salon de discussion (idempotent : un seul par review).
    /// Retourne `(salon, created)` — `created = false` si un salon existait
    /// deja (on renvoie l'existant).
    async fn create_discussion(
        &self,
        d: NewDiscussionChannel,
    ) -> Result<(DiscussionChannel, bool), DomainError>;

    /// Supprime l'enregistrement du salon de discussion d'une review (pas le
    /// transcript). Utilise quand le salon Discord a disparu (supprime a la
    /// main) pour permettre d'en rouvrir un neuf. No-op si rien a supprimer.
    async fn delete_discussion(&self, review_id: Uuid) -> Result<(), DomainError>;

    /// Persiste un lot de messages du salon de discussion (transcript).
    /// Idempotent par (review_id, discord_message_id). Retourne le nombre
    /// reellement insere (les doublons sont ignores).
    async fn append_discussion_messages(
        &self,
        messages: &[DiscussionMessage],
    ) -> Result<u64, DomainError>;

    /// Liste le transcript d'une review (ordre chronologique).
    async fn list_discussion_messages(
        &self,
        review_id: Uuid,
    ) -> Result<Vec<DiscussionMessage>, DomainError>;
}
