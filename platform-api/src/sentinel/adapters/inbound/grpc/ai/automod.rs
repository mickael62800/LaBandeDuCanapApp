//! Implementation gRPC du `AutomodService` (Phase 7A).
//! Wrappe `AnalyzeMessageUseCase`. Hot path le plus chaud : un appel par
//! message Discord recu sur les serveurs.

use std::sync::Arc;

use platform_proto::sentinel::automod::v1 as proto;
use platform_proto::sentinel::automod::v1::automod_service_server::AutomodService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use crate::sentinel::adapters::outbound::ws::broadcaster::EventBroadcaster;
use platform_core::sentinel::domain::entities::ai::message_analysis::MessageAnalysis;
use platform_core::sentinel::domain::entities::moderation::detection_flags::DetectionFlags;
use platform_core::sentinel::domain::enums::moderation::action::Action;
use platform_core::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageCommand;
use platform_core::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageUseCase;
use platform_core::sentinel::ports::inbound::ai::analyze_message::ContextMessageEntry;
use platform_core::sentinel::ports::outbound::moderation::adaptive_slowmode_repository::AdaptiveSlowmodeRepository;
pub struct AutomodGrpc {
    pub uc: Arc<dyn AnalyzeMessageUseCase>,
    pub broadcaster: Arc<EventBroadcaster>,
    /// Miroir de l'ensemble des salons en slowmode adaptatif. Le tracker du
    /// bot est en memoire : sans cette persistance, un redemarrage laisserait
    /// les salons bloques faute de savoir lesquels relacher.
    pub adaptive_slowmode_repo: Arc<dyn AdaptiveSlowmodeRepository>,
}

#[tonic::async_trait]
impl AutomodService for AutomodGrpc {
    async fn analyze_message(
        &self,
        request: Request<proto::AnalyzeMessageRequest>,
    ) -> Result<Response<proto::AnalyzeMessageResponse>, Status> {
        let req = request.into_inner();

        // Validation inputs obligatoires.
        if req.guild_id.is_empty() || req.guild_id.len() > 20 {
            return Err(Status::invalid_argument("guild_id invalide"));
        }
        if req.user_id.is_empty() || req.user_id.len() > 20 {
            return Err(Status::invalid_argument("user_id invalide"));
        }
        if req.content.is_empty() {
            return Err(Status::invalid_argument("content ne peut pas etre vide"));
        }

        // Capture pour le broadcast WS (le live tail web de l'historique
        // d'analyse) avant que `req` ne soit consomme dans la commande.
        let guild_id_evt = req.guild_id.clone();
        let username_evt = req.username.clone();

        let flags = req.flags.map(proto_to_flags).unwrap_or(DetectionFlags {
            spam: false,
            insult: false,
            profanity: false,
            link: false,
            phishing: false,
        });
        let context_messages = req
            .context_messages
            .into_iter()
            .map(|m| ContextMessageEntry {
                username: m.username,
                content: m.content,
            })
            .collect();
        let analysis = self
            .uc
            .analyze(AnalyzeMessageCommand {
                guild_id: req.guild_id.into(),
                channel_id: req.channel_id.into(),
                user_id: req.user_id.into(),
                username: req.username,
                content: req.content,
                flags,
                message_id: req.message_id.into(),
                timestamp: req.timestamp,
                context_messages,
            })
            .await
            .map_err(domain_to_status)?;

        // Metrique : compte les decisions de routage automod (observabilite —
        // taux carte/auto/rien, sevères, suppressions de lien). Scrapé via /metrics.
        use platform_core::sentinel::domain::services::moderation::automod_routing::Routing;
        let route_label = match analysis.route {
            Routing::None => "none",
            Routing::Card => "card",
            Routing::Auto => "auto",
        };
        metrics::counter!(
            "automod_decisions_total",
            "route" => route_label,
            "severe" => if analysis.severe { "true" } else { "false" },
            "link_delete" => if analysis.auto_delete_link { "true" } else { "false" },
        )
        .increment(1);

        // Push WS : previent le dashboard (historique d'analyse) en temps reel
        // quand une action est prise, au lieu d'un polling cote web.
        if analysis.action != Action::None {
            self.broadcaster.broadcast(
                "infraction_new",
                serde_json::json!({
                    "guild_id": guild_id_evt,
                    "username": username_evt,
                    "action": analysis.action.as_str(),
                    "reason": &analysis.reason,
                }),
            );
        }

        Ok(Response::new(analysis_to_proto(analysis)))
    }

    async fn evaluate_flood(
        &self,
        request: Request<proto::EvaluateFloodRequest>,
    ) -> Result<Response<proto::EvaluateFloodResponse>, Status> {
        let req = request.into_inner();
        let decision = self
            .uc
            .evaluate_flood(&req.guild_id, req.flood_count)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::EvaluateFloodResponse {
            severe: decision.severe,
            mute_duration_secs: decision.mute_duration_secs,
            score: decision.score,
        }))
    }

    async fn evaluate_caps(
        &self,
        request: Request<proto::EvaluateCapsRequest>,
    ) -> Result<Response<proto::EvaluateCapsResponse>, Status> {
        let req = request.into_inner();
        if req.guild_id.is_empty() || req.guild_id.len() > 20 {
            return Err(Status::invalid_argument("guild_id invalide"));
        }
        let decision = self
            .uc
            .evaluate_caps(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::EvaluateCapsResponse {
            score: decision.score,
        }))
    }

    async fn evaluate_attachments(
        &self,
        request: Request<proto::EvaluateAttachmentsRequest>,
    ) -> Result<Response<proto::EvaluateAttachmentsResponse>, Status> {
        let req = request.into_inner();
        if req.guild_id.is_empty() || req.guild_id.len() > 20 {
            return Err(Status::invalid_argument("guild_id invalide"));
        }
        let decision = self
            .uc
            .evaluate_attachments(&req.guild_id, req.filenames)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::EvaluateAttachmentsResponse {
            suspicious: decision.suspicious,
            action: action_to_proto(decision.action),
            reason: decision.reason,
            score: decision.score,
            filename: decision.filename,
        }))
    }

    // ── Slowmode adaptatif ──

    async fn mark_adaptive_slowmode(
        &self,
        request: Request<proto::AdaptiveSlowmodeChannel>,
    ) -> Result<Response<proto::AdaptiveSlowmodeAck>, Status> {
        let req = request.into_inner();
        if req.channel_id.is_empty() {
            return Err(Status::invalid_argument("channel_id requis"));
        }
        self.adaptive_slowmode_repo
            .mark(&req.guild_id, &req.channel_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::AdaptiveSlowmodeAck {}))
    }

    async fn unmark_adaptive_slowmode(
        &self,
        request: Request<proto::AdaptiveSlowmodeChannel>,
    ) -> Result<Response<proto::AdaptiveSlowmodeAck>, Status> {
        let req = request.into_inner();
        if req.channel_id.is_empty() {
            return Err(Status::invalid_argument("channel_id requis"));
        }
        // Cle par salon : `guild_id` n'est pas relu, la contrainte d'unicite
        // porte sur le salon.
        self.adaptive_slowmode_repo
            .unmark(&req.channel_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::AdaptiveSlowmodeAck {}))
    }

    async fn list_adaptive_slowmode(
        &self,
        _request: Request<proto::ListAdaptiveSlowmodeRequest>,
    ) -> Result<Response<proto::ListAdaptiveSlowmodeResponse>, Status> {
        let rows = self
            .adaptive_slowmode_repo
            .list_all()
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ListAdaptiveSlowmodeResponse {
            channels: rows
                .into_iter()
                .map(|(guild_id, channel_id)| proto::AdaptiveSlowmodeChannel {
                    guild_id,
                    channel_id,
                })
                .collect(),
        }))
    }
}

fn proto_to_flags(p: proto::DetectionFlags) -> DetectionFlags {
    DetectionFlags {
        spam: p.spam,
        insult: p.insult,
        profanity: p.profanity,
        link: p.link,
        phishing: p.phishing,
    }
}

fn action_to_proto(a: Action) -> i32 {
    match a {
        Action::None => proto::Action::None as i32,
        Action::Warn => proto::Action::Warn as i32,
        Action::Delete => proto::Action::Delete as i32,
        Action::Mute => proto::Action::Mute as i32,
        Action::Kick => proto::Action::Kick as i32,
        Action::Ban => proto::Action::Ban as i32,
    }
}

fn routing_to_proto(
    r: platform_core::sentinel::domain::services::moderation::automod_routing::Routing,
) -> i32 {
    use platform_core::sentinel::domain::services::moderation::automod_routing::Routing;
    match r {
        Routing::None => proto::Routing::None as i32,
        Routing::Card => proto::Routing::Card as i32,
        Routing::Auto => proto::Routing::Auto as i32,
    }
}

fn analysis_to_proto(a: MessageAnalysis) -> proto::AnalyzeMessageResponse {
    proto::AnalyzeMessageResponse {
        action: action_to_proto(a.action),
        reason: a.reason,
        score: a.score,
        duration: a.duration,
        route: routing_to_proto(a.route),
        severe: a.severe,
        auto_delete_link: a.auto_delete_link,
        auto_action: a.auto_action,
    }
}

#[cfg(test)]
#[path = "tests/automod.rs"]
mod tests;
