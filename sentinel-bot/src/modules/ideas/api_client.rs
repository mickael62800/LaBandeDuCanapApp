//! Client du module idees.
//!
//! Entierement en gRPC (`IdeasService`). Le bot ne touche jamais la base :
//! toute la regle metier (quota, transitions de statut) vit dans
//! `ManageIdeasUseCase` cote API. Les routes HTTP `/api/ideas/*` restent en
//! place pour le dashboard web, qui a besoin de la liste filtree et du detail.

use std::sync::Arc;

use crate::shared::grpc_client::{grpc_err_to_string, GrpcCallError, SentinelGrpcClient};
use platform_proto::sentinel::ideas::v1 as proto;

/// Idee telle que le bot la manipule. Miroir du sous-ensemble expose par le
/// service gRPC : les horodatages et l'identifiant du decideur restent cote
/// API, seul le web les affiche.
#[derive(Debug, Clone)]
pub struct Idea {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub category: String,
    pub author_id: String,
    pub author_name: String,
    pub decided_by_name: Option<String>,
    pub decision_reason: Option<String>,
}

impl From<proto::Idea> for Idea {
    fn from(i: proto::Idea) -> Self {
        Self {
            id: i.id,
            title: i.title,
            description: i.description,
            status: i.status,
            category: i.category,
            author_id: i.author_id,
            author_name: i.author_name,
            decided_by_name: i.decided_by_name,
            decision_reason: i.decision_reason,
        }
    }
}

/// Parametres de creation d'une idee (issus de la modale Discord).
pub struct CreateIdeaRequest {
    pub guild_id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub author_id: String,
    pub author_name: String,
    pub channel_id: Option<String>,
}

pub struct ApiClient {
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { grpc }
    }

    pub async fn create_idea(&self, req: &CreateIdeaRequest) -> Result<Idea, String> {
        let request = proto::CreateIdeaRequest {
            guild_id: req.guild_id.clone(),
            title: req.title.clone(),
            description: req.description.clone(),
            category: req.category.clone(),
            author_id: req.author_id.clone(),
            author_name: req.author_name.clone(),
            channel_id: req.channel_id.clone(),
        };
        let resp = crate::grpc_call!(self.grpc, ideas, create_idea, request)?;
        Ok(resp.into())
    }

    /// Nombre d'idees non tranchees de ce membre sur cette guild.
    pub async fn open_count(&self, guild_id: &str, author_id: &str) -> Result<i64, String> {
        let request = proto::CountOpenByAuthorRequest {
            guild_id: guild_id.to_string(),
            author_id: author_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, ideas, count_open_by_author, request)?;
        Ok(resp.open_count)
    }

    pub async fn idea_by_channel(&self, channel_id: &str) -> Result<Idea, String> {
        let request = proto::GetIdeaByChannelRequest {
            channel_id: channel_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, ideas, get_idea_by_channel, request)?;
        Ok(resp.into())
    }

    pub async fn decide(
        &self,
        idea_id: &str,
        status: &str,
        decided_by: &str,
        decided_by_name: &str,
        reason: Option<&str>,
    ) -> Result<Idea, String> {
        let request = proto::DecideIdeaRequest {
            idea_id: idea_id.to_string(),
            status: status.to_string(),
            decided_by: decided_by.to_string(),
            decided_by_name: decided_by_name.to_string(),
            reason: reason.map(str::to_string),
        };
        let resp = crate::grpc_call!(self.grpc, ideas, decide_idea, request)?;
        Ok(resp.into())
    }

    pub async fn set_channel(&self, idea_id: &str, channel_id: Option<&str>) -> Result<(), String> {
        let request = proto::SetIdeaChannelRequest {
            idea_id: idea_id.to_string(),
            channel_id: channel_id.map(str::to_string),
        };
        crate::grpc_call!(@unit self.grpc, ideas, set_idea_channel, request)
    }

    /// Sync best-effort d'un message du salon : une perte n'est pas bloquante,
    /// le salon Discord reste la source de verite pour la conversation.
    pub async fn add_message(
        &self,
        idea_id: &str,
        author_name: &str,
        author_role: &str,
        content: &str,
    ) {
        let request = proto::AddIdeaMessageRequest {
            idea_id: idea_id.to_string(),
            author_name: author_name.to_string(),
            author_role: author_role.to_string(),
            content: content.to_string(),
        };
        let res: Result<(), GrpcCallError> =
            crate::grpc_call!(@raw_unit self.grpc, ideas, add_idea_message, request);
        if let Err(e) = res {
            tracing::warn!(error = %e, idea_id = %idea_id, "sync message idee echouee (best-effort)");
        }
    }
}
