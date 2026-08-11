use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::shared::api_client::BaseApiClient;
use crate::shared::grpc_client::SentinelGrpcClient;

use sentinel_proto::moderation::v1 as proto_mod;
use sentinel_proto::sursis::v1 as proto_sursis;

/// Action de moderation envoyee au backend.
#[derive(Debug, Serialize)]
pub struct ModerationAction {
    pub guild_id: String,
    pub channel_id: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    /// Gravite pour les warns : "low", "medium", "high"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gravity: Option<String>,
    /// Duree en secondes (None = permanent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

#[derive(Debug, Deserialize)]
/// Action de moderation renvoyee par l'API. `target_name` n'est pas relu par
/// le bot (il l'a deja au moment d'agir) mais fait partie du contrat : le
/// dashboard web l'affiche dans l'historique.
#[allow(dead_code)]
pub struct ModerationActionResponse {
    pub id: String,
    pub action_type: String,
    pub target_name: String,
    pub moderator_name: String,
    pub reason: String,
    pub gravity: Option<String>,
    pub created_at: String,
    pub escalation_action: Option<String>,
    pub escalation_duration: Option<u64>,
    pub strikes_count: Option<u32>,
}

/// Historique des sanctions d'un utilisateur.
#[derive(Debug, Deserialize)]
/// Dossier d'un membre. Le bot affiche les compteurs et le nom ; `target_id`
/// n'est pas relu (il est deja connu de l'appelant) mais fait partie du contrat
/// de l'API, ou le dashboard web s'en sert.
#[allow(dead_code)]
pub struct UserHistory {
    pub target_id: String,
    pub target_name: String,
    pub total_warns: u32,
    pub total_mutes: u32,
    pub total_bans: u32,
    pub actions: Vec<ModerationActionResponse>,
}

/// MOD #2 — Preuve attachee a une action de moderation.
///
/// Reduit aux champs rendus par `/evidence` ; le message proto en porte
/// davantage (identifiant, auteur nomme, horodatage) pour le dashboard web.
#[derive(Debug)]
pub struct EvidenceEntry {
    pub action_id: String,
    pub url: String,
    pub description: Option<String>,
    pub uploaded_by: String,
}

/// MOD #3 — Entree de la file de relecture.
///
/// Reduit aux champs rendus par `/review add` et `/review list` ; le message
/// proto porte tout le dossier (relecteur, statut, horodatages) pour le web.
#[derive(Debug)]
pub struct ReviewQueueEntry {
    pub id: String,
    pub action_id: String,
    pub added_by: String,
    pub reason: Option<String>,
    /// Enrichis par jointure ; `None` si l'action a disparu entre-temps.
    pub action_type: Option<String>,
    pub target_name: Option<String>,
}

/// Faits Discord d'une cible envoyes a l'API pour l'evaluation de risque.
#[derive(Debug)]
pub struct TargetRiskFacts {
    pub account_age_days: i64,
    pub is_bot: bool,
    pub has_mod_perms: bool,
}

/// Decision de risque renvoyee par l'API (seuil + politique server-side).
#[derive(Debug)]
pub struct TargetRiskDecision {
    pub risky: bool,
    pub reason: Option<String>,
}

/// Vue bot d'un « ban en sursis » (sous-ensemble consomme).
#[derive(Debug, Clone)]
pub struct SursisData {
    pub id: String,
    pub user_id: String,
    pub saved_roles: Vec<String>,
    /// Echeance (RFC3339) affichee dans le panneau modo.
    pub expires_at: String,
}

impl From<proto_sursis::Sursis> for SursisData {
    fn from(s: proto_sursis::Sursis) -> Self {
        Self {
            id: s.id,
            user_id: s.user_id,
            saved_roles: s.saved_roles,
            expires_at: s.expires_at,
        }
    }
}

/// Parametres de mise en sursis (le bot fournit les roles sauvegardes + salon).
pub struct CreateSursisParams<'a> {
    pub guild_id: &'a str,
    pub user_id: &'a str,
    pub username: &'a str,
    pub moderator_id: &'a str,
    pub moderator_name: &'a str,
    pub reason: &'a str,
    pub saved_roles: Vec<String>,
    pub channel_id: Option<String>,
}

/// Client API de la moderation.
pub struct ApiClient {
    base: Arc<BaseApiClient>,
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>, grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { base, grpc }
    }

    /// Enregistre une action de moderation dans le backend (gRPC).
    pub async fn log_action(
        &self,
        action: &ModerationAction,
    ) -> Result<ModerationActionResponse, String> {
        self.log_action_inner(action, false).await
    }

    /// Variante d'escalade auto : journalise l'action SANS rejouer de strike.
    /// Le strike declencheur a deja ete compte par le `/warn` d'origine, donc
    /// re-striker ici creerait une boucle d'escalade (double-strike).
    pub async fn log_action_no_strike(
        &self,
        action: &ModerationAction,
    ) -> Result<ModerationActionResponse, String> {
        self.log_action_inner(action, true).await
    }

    async fn log_action_inner(
        &self,
        action: &ModerationAction,
        skip_strike: bool,
    ) -> Result<ModerationActionResponse, String> {
        let req = proto_mod::LogActionRequest {
            guild_id: action.guild_id.clone(),
            channel_id: action.channel_id.clone(),
            moderator_id: action.moderator_id.clone(),
            moderator_name: action.moderator_name.clone(),
            target_id: action.target_id.clone(),
            target_name: action.target_name.clone(),
            action_type: action.action_type.clone(),
            reason: action.reason.clone(),
            gravity: action.gravity.clone(),
            duration: action.duration,
            skip_strike,
        };
        let resp = crate::grpc_call!(self.grpc, moderation, log_action, req)?;
        Ok(ModerationActionResponse {
            id: resp.id,
            action_type: resp.action_type,
            target_name: resp.target_name,
            moderator_name: resp.moderator_name,
            reason: resp.reason,
            gravity: resp.gravity,
            created_at: resp.created_at,
            escalation_action: resp.escalation_action,
            escalation_duration: resp.escalation_duration,
            strikes_count: resp.strikes_count,
        })
    }

    /// Recupere l'historique des sanctions d'un utilisateur (gRPC).
    pub async fn get_history(&self, guild_id: &str, user_id: &str) -> Result<UserHistory, String> {
        let req = proto_mod::GetHistoryRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let history = crate::grpc_call!(self.grpc, moderation, get_history, req)?;
        Ok(UserHistory {
            target_id: history.target_id,
            target_name: history.target_name,
            total_warns: history.total_warns,
            total_mutes: history.total_mutes,
            total_bans: history.total_bans,
            actions: history
                .actions
                .into_iter()
                .map(|a| ModerationActionResponse {
                    id: a.id,
                    action_type: a.action_type,
                    target_name: a.target_name,
                    moderator_name: a.moderator_name,
                    reason: a.reason,
                    gravity: a.gravity,
                    created_at: a.created_at,
                    escalation_action: None,
                    escalation_duration: None,
                    strikes_count: None,
                })
                .collect(),
        })
    }

    /// Copilote de moderation (gRPC) : contexte d'un membre + suggestion de
    /// sanction proportionnee et explicable. Lecture seule, consultatif.
    /// Evalue server-side le risque d'une cible (garde-fou UX confirmation).
    /// Le bot fournit les FAITS Discord ; l'API applique le seuil + la politique
    /// et renvoie `{risky, reason}`.
    pub async fn assess_target_risk(
        &self,
        guild_id: &str,
        facts: &TargetRiskFacts,
    ) -> Result<TargetRiskDecision, String> {
        let req = proto_mod::AssessTargetRiskRequest {
            guild_id: guild_id.to_string(),
            account_age_days: facts.account_age_days,
            is_bot: facts.is_bot,
            has_mod_perms: facts.has_mod_perms,
        };
        let d = crate::grpc_call!(self.grpc, moderation, assess_target_risk, req)?;
        Ok(TargetRiskDecision {
            risky: d.risky,
            reason: d.reason,
        })
    }

    /// Nombre d'actions de moderation posees par ce moderateur sur la fenetre.
    /// Sert au garde-fou "quota par moderateur".
    pub async fn mod_action_count(
        &self,
        guild_id: &str,
        moderator_id: &str,
        window_secs: u64,
    ) -> Result<u32, String> {
        let req = proto_mod::CountModeratorActionsRequest {
            guild_id: guild_id.to_string(),
            moderator_id: moderator_id.to_string(),
            window_secs,
        };
        let r = crate::grpc_call!(self.grpc, moderation, count_moderator_actions, req)?;
        Ok(r.count)
    }

    /// Annule une action de moderation par son ID (`/unwarn`).
    ///
    /// L'API applique l'effet Discord inverse (unban, retrait de timeout) et
    /// annule le rappel d'auto-unban : le bot n'a rien a orchestrer.
    pub async fn delete_action(&self, action_id: &str) -> Result<bool, String> {
        let req = proto_mod::CancelActionRequest {
            action_id: action_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, moderation, cancel_action, req)?;
        Ok(resp.cancelled)
    }

    /// MOD #2 — Attache une preuve a une action de moderation existante.
    pub async fn add_evidence(
        &self,
        action_id: &str,
        url: &str,
        description: Option<&str>,
        uploaded_by: &str,
        uploaded_by_name: &str,
    ) -> Result<EvidenceEntry, String> {
        let req = proto_mod::AddEvidenceRequest {
            action_id: action_id.to_string(),
            url: url.to_string(),
            description: description.map(str::to_string),
            uploaded_by: uploaded_by.to_string(),
            uploaded_by_name: uploaded_by_name.to_string(),
        };
        let e = crate::grpc_call!(self.grpc, moderation, add_evidence, req)?;
        Ok(evidence_from_proto(e))
    }

    /// MOD #2 — Liste les preuves attachees a une action.
    pub async fn list_evidence(&self, action_id: &str) -> Result<Vec<EvidenceEntry>, String> {
        let req = proto_mod::ListEvidenceRequest {
            action_id: action_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, moderation, list_evidence, req)?;
        Ok(resp.entries.into_iter().map(evidence_from_proto).collect())
    }

    /// MOD #3 — Ajoute une action a la file de relecture.
    pub async fn add_review(
        &self,
        action_id: &str,
        guild_id: &str,
        added_by: &str,
        added_by_name: &str,
        reason: Option<&str>,
    ) -> Result<ReviewQueueEntry, String> {
        let req = proto_mod::AddReviewRequest {
            action_id: action_id.to_string(),
            guild_id: guild_id.to_string(),
            added_by: added_by.to_string(),
            added_by_name: added_by_name.to_string(),
            reason: reason.map(str::to_string),
        };
        let e = crate::grpc_call!(self.grpc, moderation, add_review, req)?;
        Ok(review_from_proto(e))
    }

    /// MOD #3 — Liste les reviews en attente d'une guild.
    pub async fn list_pending_reviews(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ReviewQueueEntry>, String> {
        let req = proto_mod::ListPendingReviewsRequest {
            guild_id: guild_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, moderation, list_pending_reviews, req)?;
        Ok(resp.entries.into_iter().map(review_from_proto).collect())
    }

    /// MOD #6 — Ecrit une cle de config bot.
    pub async fn set_bot_config(
        &self,
        guild_id: &str,
        bot_name: &str,
        config_key: &str,
        config_value: &str,
    ) {
        self.base
            .post_fire_and_forget(
                "/api/bots/config",
                &serde_json::json!({
                    "guild_id": guild_id,
                    "bot_name": bot_name,
                    "config_key": config_key,
                    "config_value": config_value,
                }),
            )
            .await;
    }

    /// MOD #3 — Resout une review en fire-and-forget.
    pub async fn resolve_review(
        &self,
        review_id: &str,
        status: &str,
        reviewer_id: &str,
        reviewer_name: &str,
        notes: Option<&str>,
    ) {
        let req = proto_mod::ResolveReviewRequest {
            review_id: review_id.to_string(),
            status: status.to_string(),
            reviewer_id: reviewer_id.to_string(),
            reviewer_name: reviewer_name.to_string(),
            notes: notes.map(str::to_string),
        };
        let res: Result<_, crate::shared::grpc_client::GrpcCallError> =
            crate::grpc_call!(@raw self.grpc, moderation, resolve_review, req);
        match res {
            Ok(r) if !r.resolved => {
                tracing::warn!(review_id = %review_id, "review introuvable a la resolution")
            }
            Err(e) => tracing::warn!(error = %e, review_id = %review_id, "echec resolution review"),
            _ => {}
        }
    }

    /// Ajoute une note sur un utilisateur.
    // ── Ban en sursis (gRPC SursisService) ──

    /// Enregistre une mise en sursis (le delai d'appel est lu cote serveur).
    pub async fn create_sursis(&self, p: CreateSursisParams<'_>) -> Result<SursisData, String> {
        let req = proto_sursis::CreateSursisRequest {
            guild_id: p.guild_id.to_string(),
            user_id: p.user_id.to_string(),
            username: p.username.to_string(),
            moderator_id: p.moderator_id.to_string(),
            moderator_name: p.moderator_name.to_string(),
            reason: p.reason.to_string(),
            saved_roles: p.saved_roles,
            channel_id: p.channel_id,
        };
        let s = crate::grpc_call!(self.grpc, sursis, create_sursis, req)?;
        Ok(s.into())
    }

    /// Recupere un sursis par son id (`NotFound` -> `Err`).
    pub async fn get_sursis(&self, id: &str) -> Result<SursisData, String> {
        let req = proto_sursis::GetSursisRequest { id: id.to_string() };
        let s = crate::grpc_call!(self.grpc, sursis, get_sursis, req)?;
        Ok(s.into())
    }

    /// Resout un sursis ("gracie" | "banni"). Retourne `claimed` (true = ce
    /// resolve a bien fait la transition ; false = deja resolu -> ne rien refaire).
    pub async fn resolve_sursis(&self, id: &str, status: &str) -> Result<bool, String> {
        let req = proto_sursis::ResolveSursisRequest {
            id: id.to_string(),
            status: status.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, sursis, resolve_sursis, req)?;
        Ok(resp.claimed)
    }

    // ── Pending Actions (mode apprenti) ──

    /// Met a jour le statut d'une action en attente (approved/rejected).
    pub async fn resolve_pending_action(&self, action_id: &str, status: &str, reviewed_by: &str) {
        let req = proto_mod::ResolvePendingActionRequest {
            action_id: action_id.to_string(),
            status: status.to_string(),
            reviewed_by: reviewed_by.to_string(),
        };
        let res: Result<(), crate::shared::grpc_client::GrpcCallError> =
            crate::grpc_call!(@raw_unit self.grpc, moderation, resolve_pending_action, req);
        if let Err(e) = res {
            tracing::warn!(error = %e, action_id = %action_id, "echec resolution action en attente");
        }
    }
}

/// Conversions proto -> types du bot. Isolees ici pour que les methodes
/// restent lisibles et que l'ajout d'un champ ne se fasse qu'a un endroit.
fn evidence_from_proto(e: proto_mod::EvidenceEntry) -> EvidenceEntry {
    EvidenceEntry {
        action_id: e.action_id,
        url: e.url,
        description: e.description,
        uploaded_by: e.uploaded_by,
    }
}

fn review_from_proto(r: proto_mod::ReviewEntry) -> ReviewQueueEntry {
    ReviewQueueEntry {
        id: r.id,
        action_id: r.action_id,
        added_by: r.added_by,
        reason: r.reason,
        action_type: r.action_type,
        target_name: r.target_name,
    }
}

use crate::shared::grpc_client::grpc_err_to_string;
