//! Client API du automod module.
//!
//! Phase 7A -- Migration gRPC complete : `analyze` est le **hot path le plus
//! chaud du projet** (un appel par message Discord recu sur tous les
//! serveurs). Le gain perf gRPC est ici maximal.
//!
//! ## Comportement si l'API tombe
//!
//! Le circuit breaker (5 echecs / 10s) court-circuite immediatement les
//! appels suivants. Pendant l'ouverture, `analyze` retourne `Err("API
//! indisponible")` et le bot **n'applique aucune action de moderation**.
//! Comportement par defaut : laisser passer le message (ne pas faire de
//! faux positifs basees sur une API down). Cote handler, le timeout
//! original de 5s est conserve pour ne pas bloquer le bot.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::shared::grpc_client::SentinelGrpcClient;

use platform_proto::sentinel::automod::v1 as proto;
use platform_proto::sentinel::automod_review::v1 as proto_review;

use super::detectors::DetectionFlags;

/// Faits Discord du demandeur pour les regles d'acces cote core
/// (`can_finalize_review`, `is_moderator`). Reprend le 5-tuple `moderator_facts`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReviewFacts {
    pub is_admin: bool,
    pub has_moderate_members: bool,
    pub has_manage_messages: bool,
    pub has_mod_role: bool,
    pub has_admin_role: bool,
}

impl From<(bool, bool, bool, bool, bool)> for ReviewFacts {
    /// Depuis le 5-tuple de `render::moderator_facts`
    /// (is_admin, has_moderate_members, has_manage_messages, has_mod_role, has_admin_role).
    fn from(t: (bool, bool, bool, bool, bool)) -> Self {
        Self {
            is_admin: t.0,
            has_moderate_members: t.1,
            has_manage_messages: t.2,
            has_mod_role: t.3,
            has_admin_role: t.4,
        }
    }
}

impl From<ReviewFacts> for proto_review::ModeratorFacts {
    fn from(f: ReviewFacts) -> Self {
        Self {
            is_admin: f.is_admin,
            has_moderate_members: f.has_moderate_members,
            has_manage_messages: f.has_manage_messages,
            has_mod_role: f.has_mod_role,
            has_admin_role: f.has_admin_role,
        }
    }
}

/// Vue bot d'une carte de review (sous-ensemble consomme par les rendus).
#[derive(Debug, Clone, Default)]
pub struct ReviewData {
    pub id: String,
    pub merged: bool,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: String,
    pub user_id: String,
    pub user_name: String,
    pub content_preview: String,
    pub reason: String,
    pub suggested_action: String,
    pub score: f64,
    pub cumulative_score: f64,
    pub incident_count: i32,
    pub voting_deadline: Option<String>,
    pub status: String,
    pub decided_action: Option<String>,
    /// Flags de detection (objet JSON). Vide `{}` si absent/illisible.
    pub flags: serde_json::Value,
    /// Incidents agreges (tableau JSON). Vide `[]` si absent/illisible.
    pub incidents: serde_json::Value,
}

impl From<proto_review::AutomodReview> for ReviewData {
    fn from(r: proto_review::AutomodReview) -> Self {
        let flags = serde_json::from_str(&r.flags_json).unwrap_or_else(|_| serde_json::json!({}));
        let incidents =
            serde_json::from_str(&r.incidents_json).unwrap_or_else(|_| serde_json::json!([]));
        Self {
            id: r.id,
            merged: r.merged,
            guild_id: r.guild_id,
            channel_id: r.channel_id,
            message_id: r.message_id,
            user_id: r.user_id,
            user_name: r.user_name,
            content_preview: r.content_preview,
            reason: r.reason,
            suggested_action: r.suggested_action,
            score: r.score,
            cumulative_score: r.cumulative_score,
            incident_count: r.incident_count,
            voting_deadline: r.voting_deadline,
            status: r.status,
            decided_action: r.decided_action,
            flags,
            incidents,
        }
    }
}

/// Salon de discussion lie a une carte (sous-ensemble consomme par le bot).
#[derive(Debug, Clone)]
pub struct DiscussionInfo {
    pub channel_id: String,
    /// true si l'appel vient de creer l'enregistrement (false = existait deja).
    pub created: bool,
}

impl From<proto_review::DiscussionChannel> for DiscussionInfo {
    fn from(d: proto_review::DiscussionChannel) -> Self {
        Self {
            channel_id: d.channel_id,
            created: d.created,
        }
    }
}

/// Un message de transcript a persister a l'archivage du salon.
pub struct DiscussionMessageIn {
    pub discord_message_id: String,
    pub author_id: String,
    pub author_name: String,
    pub author_is_bot: bool,
    pub content: String,
    /// RFC3339.
    pub sent_at: String,
}

/// Un vote individuel (rendu de la carte agregee). Le rendu n'affiche que le
/// nom et l'action ; `voter_id` n'est pas transporte cote bot.
#[derive(Debug, Clone)]
pub struct ReviewVote {
    pub voter_name: String,
    pub vote_action: String,
}

/// Parametres de creation d'une carte de review (mode vote).
pub struct CreateReviewParams<'a> {
    pub guild_id: &'a str,
    pub channel_id: &'a str,
    pub message_id: &'a str,
    pub user_id: &'a str,
    pub user_name: &'a str,
    pub content_preview: &'a str,
    pub suggested_action: &'a str,
    pub score: f64,
    pub reason: &'a str,
    /// JSON objet des flags de detection.
    pub flags: serde_json::Value,
    /// RFC3339 ; `None` = pas de mode vote.
    pub voting_deadline: Option<String>,
    pub aggregate: bool,
    pub aggregate_window_minutes: Option<i64>,
    pub already_sanctioned: bool,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeRequest {
    pub guild_id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub content: String,
    pub flags: DetectionFlags,
    pub metadata: MessageMetadata,
    pub context_messages: Vec<ContextMessage>,
}

#[derive(Debug, Serialize)]
pub struct ContextMessage {
    pub username: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct MessageMetadata {
    pub message_id: String,
    pub timestamp: String,
}

/// Decision de routage calculee cote serveur (decide = API). Le bot execute.
#[derive(Debug, Deserialize, PartialEq, Clone, Copy)]
pub enum Routing {
    /// Ne rien faire automatiquement.
    None,
    /// Poster une carte de review/vote.
    Card,
    /// Appliquer directement l'action (mode auto).
    Auto,
}

#[derive(Debug, Deserialize)]
pub struct AnalyzeResponse {
    pub action: Action,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub score: Option<f64>,
    /// Decision de routage (cote serveur).
    pub route: Routing,
    /// Cas severe -> protection auto (mute + suppression) immediate.
    pub severe: bool,
    /// Lien non autorise hors image -> suppression auto immediate.
    pub auto_delete_link: bool,
    /// Executer la sanction, y compris quand une carte est aussi creee.
    pub auto_action: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    None,
    Warn,
    Delete,
    Mute,
    Kick,
    Ban,
}

pub struct ApiClient {
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { grpc }
    }

    /// Client gRPC sous-jacent, pour les helpers de sync transverse
    /// (`crate::sync`) qui ne passent pas par un ApiClient de module.
    pub fn grpc(&self) -> &Arc<SentinelGrpcClient> {
        &self.grpc
    }

    /// gRPC `AutomodService.AnalyzeMessage` (hot path le plus chaud).
    pub async fn analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzeResponse, String> {
        let req = proto::AnalyzeMessageRequest {
            guild_id: request.guild_id.clone(),
            channel_id: request.channel_id.clone(),
            user_id: request.user_id.clone(),
            username: request.username.clone(),
            content: request.content.clone(),
            flags: Some(proto::DetectionFlags {
                spam: request.flags.spam,
                insult: request.flags.insult,
                profanity: request.flags.profanity,
                link: request.flags.link,
                phishing: request.flags.phishing,
            }),
            message_id: request.metadata.message_id.clone(),
            timestamp: request.metadata.timestamp.clone(),
            context_messages: request
                .context_messages
                .iter()
                .map(|m| proto::ContextMessage {
                    username: m.username.clone(),
                    content: m.content.clone(),
                })
                .collect(),
        };
        let resp = crate::grpc_call!(self.grpc, automod, analyze_message, req)?;
        Ok(AnalyzeResponse {
            action: proto_action_to_action(resp.action),
            reason: if resp.reason.is_empty() {
                None
            } else {
                Some(resp.reason)
            },
            score: Some(resp.score),
            route: proto_routing_to_routing(resp.route),
            severe: resp.severe,
            auto_delete_link: resp.auto_delete_link,
            auto_action: resp.auto_action,
        })
    }

    /// gRPC `AutomodService.EvaluateFlood` : verdict d'auto-protection face a
    /// un flood. Retourne `(severe, mute_duration_secs)`. La regle (seuil
    /// severe + toggle) vit cote serveur.
    pub async fn evaluate_flood(
        &self,
        guild_id: &str,
        user_id: &str,
        channel_id: &str,
        flood_count: i32,
    ) -> Result<(bool, i64, f64), String> {
        let req = proto::EvaluateFloodRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            channel_id: channel_id.to_string(),
            flood_count,
        };
        let resp = crate::grpc_call!(self.grpc, automod, evaluate_flood, req)?;
        Ok((resp.severe, resp.mute_duration_secs, resp.score))
    }

    /// gRPC `AutomodService.EvaluateAttachments` : verdict sur des pieces
    /// jointes suspectes. La regle (extensions dangereuses + config) vit cote
    /// serveur ; le bot n'EXECUTE que l'action renvoyee.
    pub async fn evaluate_attachments(
        &self,
        guild_id: &str,
        filenames: Vec<String>,
    ) -> Result<AttachmentVerdict, String> {
        let req = proto::EvaluateAttachmentsRequest {
            guild_id: guild_id.to_string(),
            filenames,
        };
        let resp = crate::grpc_call!(self.grpc, automod, evaluate_attachments, req)?;
        Ok(AttachmentVerdict {
            suspicious: resp.suspicious,
            action: proto_action_to_action(resp.action),
            reason: resp.reason,
            score: resp.score,
            filename: resp.filename,
        })
    }

    /// gRPC `AutomodService.EvaluateCaps` : score de confiance a afficher pour
    /// une detection de CAPS. La detection reste locale (rate/forme) ; le SCORE
    /// affiche est fabrique cote serveur (avant : 0.8 code en dur dans le bot).
    pub async fn evaluate_caps(&self, guild_id: &str) -> Result<f64, String> {
        let req = proto::EvaluateCapsRequest {
            guild_id: guild_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, automod, evaluate_caps, req)?;
        Ok(resp.score)
    }

    // analyze_image supprime -- migre vers ai-worker (async queue + Redis).

    // ── Cartes de review / vote (gRPC AutomodReviewService, tranche 1) ──

    /// Cree (ou agrege) une carte de review en mode vote. Retourne la carte
    /// (avec `merged` = true si l'incident a fusionne dans une carte existante).
    pub async fn create_review(&self, p: CreateReviewParams<'_>) -> Result<ReviewData, String> {
        let flags_json =
            serde_json::to_string(&p.flags).map_err(|e| format!("serialisation flags: {e}"))?;
        let req = proto_review::CreateReviewRequest {
            guild_id: p.guild_id.to_string(),
            channel_id: p.channel_id.to_string(),
            message_id: p.message_id.to_string(),
            user_id: p.user_id.to_string(),
            user_name: p.user_name.to_string(),
            content_preview: p.content_preview.to_string(),
            suggested_action: p.suggested_action.to_string(),
            score: p.score,
            reason: p.reason.to_string(),
            flags_json,
            voting_deadline: p.voting_deadline,
            aggregate: p.aggregate,
            aggregate_window_minutes: p.aggregate_window_minutes,
            already_sanctioned: p.already_sanctioned,
        };
        let r = crate::grpc_call!(self.grpc, automod_review, create_review, req)?;
        Ok(r.into())
    }

    /// Recupere une carte par son id. `NotFound` -> `Err`.
    pub async fn get_review(&self, review_id: &str) -> Result<ReviewData, String> {
        let req = proto_review::GetReviewRequest {
            review_id: review_id.to_string(),
        };
        let r = crate::grpc_call!(self.grpc, automod_review, get_review, req)?;
        Ok(r.into())
    }

    /// Retrouve la carte liee a un message Discord (`None` si absente).
    pub async fn find_review_by_message(
        &self,
        guild_id: &str,
        message_id: &str,
    ) -> Result<Option<ReviewData>, String> {
        let req = proto_review::FindReviewByMessageRequest {
            guild_id: guild_id.to_string(),
            message_id: message_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, automod_review, find_review_by_message, req)?;
        Ok(resp.review.map(Into::into))
    }

    /// Finalise une carte (source discord). La sanction de membre est
    /// journalisee cote API dans le meme appel.
    pub async fn resolve_review(
        &self,
        review_id: &str,
        applied_action: &str,
        resolved_by_id: &str,
        resolved_by_name: &str,
        facts: ReviewFacts,
    ) -> Result<ReviewData, String> {
        let req = proto_review::ResolveReviewRequest {
            review_id: review_id.to_string(),
            applied_action: applied_action.to_string(),
            resolved_by_id: resolved_by_id.to_string(),
            resolved_by_name: resolved_by_name.to_string(),
            requester: Some(facts.into()),
        };
        let r = crate::grpc_call!(self.grpc, automod_review, resolve_review, req)?;
        Ok(r.into())
    }

    /// Clore immediatement en "ignore".
    pub async fn ignore_review(
        &self,
        review_id: &str,
        actor_id: &str,
        actor_name: &str,
        facts: ReviewFacts,
    ) -> Result<ReviewData, String> {
        let req = proto_review::IgnoreReviewRequest {
            review_id: review_id.to_string(),
            actor_id: actor_id.to_string(),
            actor_name: actor_name.to_string(),
            requester: Some(facts.into()),
        };
        let r = crate::grpc_call!(self.grpc, automod_review, ignore_review, req)?;
        Ok(r.into())
    }

    /// Rouvrir un dossier resolu/ignore (repasse en vote). `deadline_hours` = 0
    /// => defaut serveur (72h).
    pub async fn reopen_review(
        &self,
        review_id: &str,
        actor_id: &str,
        actor_name: &str,
        deadline_hours: i64,
        facts: ReviewFacts,
    ) -> Result<ReviewData, String> {
        let req = proto_review::ReopenReviewRequest {
            review_id: review_id.to_string(),
            actor_id: actor_id.to_string(),
            actor_name: actor_name.to_string(),
            deadline_hours,
            requester: Some(facts.into()),
        };
        let r = crate::grpc_call!(self.grpc, automod_review, reopen_review, req)?;
        Ok(r.into())
    }

    /// Enregistre un vote et retourne la liste des votes a jour.
    pub async fn vote(
        &self,
        review_id: &str,
        voter_id: &str,
        voter_name: &str,
        vote_action: &str,
        facts: ReviewFacts,
    ) -> Result<Vec<ReviewVote>, String> {
        let req = proto_review::VoteRequest {
            review_id: review_id.to_string(),
            voter_id: voter_id.to_string(),
            voter_name: voter_name.to_string(),
            vote_action: vote_action.to_string(),
            requester: Some(facts.into()),
        };
        let list = crate::grpc_call!(self.grpc, automod_review, vote, req)?;
        Ok(list.votes.into_iter().map(vote_from_proto).collect())
    }

    /// Liste les votes d'une carte.
    pub async fn list_votes(&self, review_id: &str) -> Result<Vec<ReviewVote>, String> {
        let req = proto_review::ListVotesRequest {
            review_id: review_id.to_string(),
        };
        let list = crate::grpc_call!(self.grpc, automod_review, list_votes, req)?;
        Ok(list.votes.into_iter().map(vote_from_proto).collect())
    }

    // ── Salons de discussion (tranche 2) ──

    /// Salon de discussion deja enregistre pour cette carte (`None` si aucun).
    pub async fn get_discussion(&self, review_id: &str) -> Result<Option<DiscussionInfo>, String> {
        let req = proto_review::GetDiscussionRequest {
            review_id: review_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, automod_review, get_discussion, req)?;
        Ok(resp.channel.map(Into::into))
    }

    /// Enregistre (idempotent) le salon apres la regle d'acces cote core.
    /// `created` = false => un salon existait deja.
    pub async fn open_discussion(
        &self,
        review_id: &str,
        guild_id: &str,
        channel_id: &str,
        opened_by_id: &str,
        opened_by_name: &str,
        facts: ReviewFacts,
    ) -> Result<DiscussionInfo, String> {
        let req = proto_review::OpenDiscussionRequest {
            review_id: review_id.to_string(),
            guild_id: guild_id.to_string(),
            channel_id: channel_id.to_string(),
            opened_by_id: opened_by_id.to_string(),
            opened_by_name: opened_by_name.to_string(),
            requester: Some(facts.into()),
        };
        let d = crate::grpc_call!(self.grpc, automod_review, open_discussion, req)?;
        Ok(d.into())
    }

    /// Purge l'enregistrement du salon (salon Discord supprime a la main).
    pub async fn delete_discussion(&self, review_id: &str) -> Result<(), String> {
        let req = proto_review::DeleteDiscussionRequest {
            review_id: review_id.to_string(),
        };
        crate::grpc_call!(@unit self.grpc, automod_review, delete_discussion, req)
    }

    /// Persiste un lot de messages du salon (transcript). Retourne le nombre insere.
    pub async fn append_discussion_messages(
        &self,
        review_id: &str,
        messages: Vec<DiscussionMessageIn>,
    ) -> Result<u64, String> {
        let req = proto_review::AppendDiscussionMessagesRequest {
            review_id: review_id.to_string(),
            messages: messages
                .into_iter()
                .map(|m| proto_review::DiscussionMessageIn {
                    discord_message_id: m.discord_message_id,
                    author_id: m.author_id,
                    author_name: m.author_name,
                    author_is_bot: m.author_is_bot,
                    content: m.content,
                    sent_at: m.sent_at,
                })
                .collect(),
        };
        let resp = crate::grpc_call!(self.grpc, automod_review, append_discussion_messages, req)?;
        Ok(resp.inserted)
    }
}

fn vote_from_proto(v: proto_review::ReviewVote) -> ReviewVote {
    ReviewVote {
        voter_name: v.voter_name,
        vote_action: v.vote_action,
    }
}

/// Verdict d'analyse de pieces jointes renvoye par l'API.
#[derive(Debug)]
pub struct AttachmentVerdict {
    pub suspicious: bool,
    pub action: Action,
    pub reason: String,
    pub score: f64,
    pub filename: String,
}

fn proto_action_to_action(value: i32) -> Action {
    match proto::Action::try_from(value).unwrap_or(proto::Action::None) {
        proto::Action::None => Action::None,
        proto::Action::Warn => Action::Warn,
        proto::Action::Delete => Action::Delete,
        proto::Action::Mute => Action::Mute,
        proto::Action::Kick => Action::Kick,
        proto::Action::Ban => Action::Ban,
    }
}

fn proto_routing_to_routing(value: i32) -> Routing {
    match proto::Routing::try_from(value).unwrap_or(proto::Routing::None) {
        proto::Routing::None => Routing::None,
        proto::Routing::Card => Routing::Card,
        proto::Routing::Auto => Routing::Auto,
    }
}

use crate::shared::grpc_client::grpc_err_to_string;

// ── Persistance du slowmode adaptatif (BUG3) ──
// Le tracker est en memoire ; on mirroir l'ensemble actif cote API pour le
// recharger apres un redemarrage (sinon salons bloques en slowmode a vie).

/// Marque un salon comme slowmode adaptatif actif (best-effort).
pub async fn persist_slowmode(grpc: &Arc<SentinelGrpcClient>, guild_id: &str, channel_id: &str) {
    let req = proto::AdaptiveSlowmodeChannel {
        guild_id: guild_id.to_string(),
        channel_id: channel_id.to_string(),
    };
    if let Err(e) = crate::grpc_call!(@raw_unit grpc, automod, mark_adaptive_slowmode, req) {
        tracing::warn!(error = %e, channel_id, "slowmode adaptatif non persiste");
    }
}

/// Retire un salon (slowmode desactive) — best-effort.
pub async fn forget_slowmode(grpc: &Arc<SentinelGrpcClient>, channel_id: &str) {
    // `guild_id` vide : la cle de suppression est le salon, unique en base.
    let req = proto::AdaptiveSlowmodeChannel {
        guild_id: String::new(),
        channel_id: channel_id.to_string(),
    };
    if let Err(e) = crate::grpc_call!(@raw_unit grpc, automod, unmark_adaptive_slowmode, req) {
        tracing::warn!(error = %e, channel_id, "retrait du slowmode adaptatif non persiste");
    }
}

/// Salons a relacher au demarrage. Le serveur porte aussi le `guild_id` pour
/// le dashboard ; le tracker du bot ne s'indexe que par salon.
pub async fn list_slowmode(grpc: &Arc<SentinelGrpcClient>) -> Vec<String> {
    let req = proto::ListAdaptiveSlowmodeRequest {};
    match crate::grpc_call!(@raw grpc, automod, list_adaptive_slowmode, req) {
        Ok(resp) => resp.channels.into_iter().map(|c| c.channel_id).collect(),
        Err(e) => {
            // Liste vide = aucun salon relache au demarrage. On le dit, sinon
            // le symptome (slowmode fige) est indiscernable d'un ensemble
            // reellement vide.
            tracing::warn!(error = %e, "rechargement du slowmode adaptatif impossible");
            Vec::new()
        }
    }
}
