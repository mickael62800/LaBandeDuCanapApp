//! gRPC handler de la boite a idees.
//!
//! Miroir des routes HTTP `/api/ideas/*` consommees par le bot. Le web garde
//! ses propres routes HTTP (liste filtree, detail, suppression) : elles n'ont
//! pas d'equivalent ici parce que le bot ne les appelle jamais.

use std::sync::Arc;

use platform_proto::sentinel::ideas::v1 as proto;
use platform_proto::sentinel::ideas::v1::ideas_service_server::IdeasService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use platform_core::sentinel::domain::entities::community::idea::Idea;
use platform_core::sentinel::ports::inbound::community::manage_ideas::{
    AddIdeaMessageCommand, CreateIdeaCommand, DecideIdeaCommand, ManageIdeasUseCase,
};

pub struct IdeasGrpc {
    pub uc: Arc<dyn ManageIdeasUseCase>,
}

/// Projette l'entite du domaine sur le sous-ensemble expose au bot.
///
/// Les horodatages et `decided_by` ne sont pas transmis : seul le dashboard
/// web les affiche, et les faire transiter obligerait a fixer un format de
/// date dans le contrat pour rien.
fn to_proto(idea: Idea) -> proto::Idea {
    proto::Idea {
        id: idea.id.to_string(),
        title: idea.title,
        description: idea.description,
        status: idea.status,
        category: idea.category,
        author_id: idea.author_id,
        author_name: idea.author_name,
        decided_by_name: idea.decided_by_name,
        decision_reason: idea.decision_reason,
    }
}

/// Parse un identifiant d'idee, en refusant proprement un UUID malforme.
fn parse_idea_id(raw: &str) -> Result<uuid::Uuid, Status> {
    raw.parse()
        .map_err(|_| Status::invalid_argument("idea_id doit etre un UUID"))
}

#[tonic::async_trait]
impl IdeasService for IdeasGrpc {
    async fn create_idea(
        &self,
        request: Request<proto::CreateIdeaRequest>,
    ) -> Result<Response<proto::Idea>, Status> {
        let req = request.into_inner();
        if req.guild_id.is_empty() {
            return Err(Status::invalid_argument("guild_id requis"));
        }
        if req.title.trim().is_empty() {
            return Err(Status::invalid_argument("title requis"));
        }
        let idea = self
            .uc
            .create(CreateIdeaCommand {
                guild_id: req.guild_id,
                title: req.title,
                description: req.description,
                category: req.category,
                author_id: req.author_id,
                author_name: req.author_name,
                channel_id: req.channel_id,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(to_proto(idea)))
    }

    async fn count_open_by_author(
        &self,
        request: Request<proto::CountOpenByAuthorRequest>,
    ) -> Result<Response<proto::CountOpenByAuthorResponse>, Status> {
        let req = request.into_inner();
        let open_count = self
            .uc
            .count_open_by_author(&req.guild_id, &req.author_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CountOpenByAuthorResponse {
            open_count,
        }))
    }

    async fn get_idea_by_channel(
        &self,
        request: Request<proto::GetIdeaByChannelRequest>,
    ) -> Result<Response<proto::Idea>, Status> {
        let req = request.into_inner();
        // `get_by_channel` rend `Option` : un salon sans idee est un cas
        // normal cote domaine, mais une erreur `NotFound` cote appelant, qui
        // interroge parce qu'il croit avoir affaire a un salon d'idee.
        let idea = self
            .uc
            .get_by_channel(&req.channel_id)
            .await
            .map_err(domain_to_status)?
            .ok_or_else(|| Status::not_found("aucune idee pour ce salon"))?;
        Ok(Response::new(to_proto(idea)))
    }

    async fn decide_idea(
        &self,
        request: Request<proto::DecideIdeaRequest>,
    ) -> Result<Response<proto::Idea>, Status> {
        let req = request.into_inner();
        let id = parse_idea_id(&req.idea_id)?;
        let idea = self
            .uc
            .decide(DecideIdeaCommand {
                id,
                status: req.status,
                decided_by: req.decided_by,
                decided_by_name: req.decided_by_name,
                reason: req.reason,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(to_proto(idea)))
    }

    async fn set_idea_channel(
        &self,
        request: Request<proto::SetIdeaChannelRequest>,
    ) -> Result<Response<proto::Idea>, Status> {
        let req = request.into_inner();
        let id = parse_idea_id(&req.idea_id)?;
        let idea = self
            .uc
            .set_channel(id, req.channel_id.as_deref())
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(to_proto(idea)))
    }

    async fn add_idea_message(
        &self,
        request: Request<proto::AddIdeaMessageRequest>,
    ) -> Result<Response<proto::AddIdeaMessageResponse>, Status> {
        let req = request.into_inner();
        let idea_id = parse_idea_id(&req.idea_id)?;
        let message = self
            .uc
            .add_message(AddIdeaMessageCommand {
                idea_id,
                author_name: req.author_name,
                author_role: req.author_role,
                content: req.content,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::AddIdeaMessageResponse {
            message_id: message.id.to_string(),
        }))
    }
}
