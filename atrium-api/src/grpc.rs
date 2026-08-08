//! Adaptateur gRPC unaire consomme par `atrium-bot`.

use std::sync::Arc;

use atrium_core::{
    domain::{ConversationScope, WelcomeRequest},
    ports::inbound::GenerateWelcomeReplyUseCase,
};
use atrium_proto::welcome::v1::{
    self as proto,
    bot_control_service_server::{BotControlService, BotControlServiceServer},
    welcome_service_server::{WelcomeService, WelcomeServiceServer},
};
use tonic::{Request, Response, Status};

use crate::{
    budget::BudgetGuard, control::BotControlStore, memory::ConversationMemory, merge_context,
    rag::RagService, welcome_use_case, AppConfig,
};

use std::pin::Pin;
use tokio_stream::Stream;

pub async fn serve(
    config: AppConfig,
    rag: Arc<RagService>,
    budget: Arc<BudgetGuard>,
    control: Arc<BotControlStore>,
    memory: Arc<ConversationMemory>,
) {
    let addr = config.grpc_addr;
    let welcome_service = WelcomeGrpc {
        welcome: welcome_use_case(&config),
        rag: Some(rag.clone()),
        budget: Some(budget),
        control: Some(control.clone()),
        memory: Some(memory),
    };
    let rag_service = RagGrpc { rag: Some(rag) };
    let control_service = BotControlGrpc { control };
    let grpc_token = config.grpc_token.clone();
    let auth = move |request: Request<()>| {
        let expected = format!("Bearer {grpc_token}");
        if request
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            == Some(expected.as_str())
        {
            Ok(request)
        } else {
            Err(Status::unauthenticated(
                "jeton gRPC Atrium invalide ou absent",
            ))
        }
    };

    tracing::info!(%addr, "Atrium gRPC démarré (Welcome & RAG)");
    tonic::transport::Server::builder()
        .add_service(WelcomeServiceServer::with_interceptor(
            welcome_service,
            auth.clone(),
        ))
        .add_service(
            proto::rag_service_server::RagServiceServer::with_interceptor(
                rag_service,
                auth.clone(),
            ),
        )
        .add_service(BotControlServiceServer::with_interceptor(
            control_service,
            auth,
        ))
        .serve(addr)
        .await
        .expect("serveur gRPC Atrium");
}

pub struct WelcomeGrpc {
    pub welcome: Arc<dyn GenerateWelcomeReplyUseCase>,
    pub rag: Option<Arc<RagService>>,
    pub budget: Option<Arc<BudgetGuard>>,
    pub control: Option<Arc<BotControlStore>>,
    pub memory: Option<Arc<ConversationMemory>>,
}

impl WelcomeGrpc {
    pub fn new(welcome: Arc<dyn GenerateWelcomeReplyUseCase>) -> Self {
        Self {
            welcome,
            rag: None,
            budget: None,
            control: None,
            memory: None,
        }
    }

    async fn ensure_enabled(&self, guild_id: &str) -> Result<(), Status> {
        let Some(control) = &self.control else {
            return Ok(());
        };
        match control.is_enabled(guild_id).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(Status::failed_precondition("Atrium est desactive")),
            Err(error) => {
                tracing::error!(%error, "Verification de l'etat Atrium impossible");
                Err(Status::unavailable("verification de l'etat indisponible"))
            }
        }
    }

    async fn budget_message(
        &self,
        input: &proto::GenerateReplyRequest,
    ) -> Result<Option<String>, Status> {
        let Some(budget) = &self.budget else {
            return Ok(None);
        };
        budget
            .check_and_record(
                &input.guild_id,
                &input.member_id,
                !input.member_message.trim().is_empty(),
            )
            .await
            .map_err(|error| {
                tracing::error!(%error, "Verification du budget DeepSeek gRPC impossible");
                Status::unavailable("verification du quota indisponible")
            })
    }

    async fn history(&self, guild_id: &str, member_id: &str) -> Result<String, Status> {
        match &self.memory {
            Some(memory) => memory.history(guild_id, member_id).await.map_err(|error| {
                tracing::error!(%error, "Lecture de la memoire Atrium impossible");
                Status::unavailable("lecture de la memoire indisponible")
            }),
            None => Ok(String::new()),
        }
    }

    async fn remember(&self, guild_id: &str, member_id: &str, message: &str, reply: &str) {
        if let Some(memory) = &self.memory {
            if let Err(error) = memory
                .remember_exchange(guild_id, member_id, message, reply)
                .await
            {
                tracing::warn!(%error, "Sauvegarde de la memoire Atrium impossible");
            }
        }
    }
}

pub struct RagGrpc {
    pub rag: Option<Arc<RagService>>,
}

#[tonic::async_trait]
impl WelcomeService for WelcomeGrpc {
    type StreamReplyStream =
        Pin<Box<dyn Stream<Item = Result<proto::ReplyChunk, Status>> + Send + 'static>>;

    async fn generate_reply(
        &self,
        request: Request<proto::GenerateReplyRequest>,
    ) -> Result<Response<proto::GenerateReplyResponse>, Status> {
        let input = request.into_inner();
        self.ensure_enabled(&input.guild_id).await?;
        if let Some(message) = self.budget_message(&input).await? {
            return Ok(Response::new(proto::GenerateReplyResponse {
                reply: message,
                generated_by_ai: false,
            }));
        }
        let history = self.history(&input.guild_id, &input.member_id).await?;
        let guild_id = input.guild_id.clone();
        let member_id = input.member_id.clone();
        let member_message = input.member_message.clone();
        let scope = match proto::ConversationScope::try_from(input.scope)
            .unwrap_or(proto::ConversationScope::General)
        {
            proto::ConversationScope::General => ConversationScope::General,
            proto::ConversationScope::Direct => ConversationScope::Direct,
        };
        let retrieved = match &self.rag {
            Some(rag) => rag
                .context_for(&input.guild_id, &input.member_message)
                .await
                .map_err(|error| {
                    tracing::warn!(%error, "Recherche RAG gRPC indisponible");
                    Status::unavailable("recherche de connaissances indisponible")
                })?,
            None => String::new(),
        };
        let reply = self
            .welcome
            .reply(WelcomeRequest {
                guild_id: input.guild_id,
                member_id: input.member_id,
                member_display_name: input.member_display_name,
                channel_id: input.channel_id,
                scope,
                member_message: input.member_message,
                conversation_history: history,
                server_context: merge_context(&input.server_context, &retrieved),
            })
            .await
            .map_err(|error| Status::invalid_argument(error.to_string()))?;
        self.remember(&guild_id, &member_id, &member_message, &reply.content)
            .await;
        Ok(Response::new(proto::GenerateReplyResponse {
            reply: reply.content,
            generated_by_ai: reply.generated_by_ai,
        }))
    }

    async fn stream_reply(
        &self,
        request: Request<proto::GenerateReplyRequest>,
    ) -> Result<Response<Self::StreamReplyStream>, Status> {
        let input = request.into_inner();
        self.ensure_enabled(&input.guild_id).await?;
        if let Some(message) = self.budget_message(&input).await? {
            let output_stream = async_stream::try_stream! {
                yield proto::ReplyChunk {
                    delta: message,
                    is_final: true,
                };
            };
            return Ok(Response::new(Box::pin(output_stream)));
        }
        let history = self.history(&input.guild_id, &input.member_id).await?;
        let guild_id = input.guild_id.clone();
        let member_id = input.member_id.clone();
        let member_message = input.member_message.clone();
        let scope = match proto::ConversationScope::try_from(input.scope)
            .unwrap_or(proto::ConversationScope::General)
        {
            proto::ConversationScope::General => ConversationScope::General,
            proto::ConversationScope::Direct => ConversationScope::Direct,
        };
        let retrieved = match &self.rag {
            Some(rag) => rag
                .context_for(&input.guild_id, &input.member_message)
                .await
                .map_err(|error| {
                    tracing::warn!(%error, "Recherche RAG gRPC indisponible");
                    Status::unavailable("recherche de connaissances indisponible")
                })?,
            None => String::new(),
        };
        let reply = self
            .welcome
            .reply(WelcomeRequest {
                guild_id: input.guild_id,
                member_id: input.member_id,
                member_display_name: input.member_display_name,
                channel_id: input.channel_id,
                scope,
                member_message: input.member_message,
                conversation_history: history,
                server_context: merge_context(&input.server_context, &retrieved),
            })
            .await
            .map_err(|error| Status::invalid_argument(error.to_string()))?;
        self.remember(&guild_id, &member_id, &member_message, &reply.content)
            .await;

        // Simuler ou découper la réponse en tokens pour le streaming gRPC
        let content = reply.content;
        let output_stream = async_stream::try_stream! {
            let words: Vec<&str> = content.split_whitespace().collect();
            for (idx, word) in words.iter().enumerate() {
                let space = if idx > 0 { " " } else { "" };
                let delta = format!("{space}{word}");
                let is_final = idx == words.len() - 1;
                yield proto::ReplyChunk {
                    delta,
                    is_final,
                };
            }
        };

        Ok(Response::new(Box::pin(output_stream)))
    }
}

pub struct BotControlGrpc {
    control: Arc<BotControlStore>,
}

#[tonic::async_trait]
impl BotControlService for BotControlGrpc {
    async fn get_state(
        &self,
        request: Request<proto::BotStateRequest>,
    ) -> Result<Response<proto::BotStateResponse>, Status> {
        let guild_id = request.into_inner().guild_id;
        if guild_id.is_empty() {
            return Err(Status::invalid_argument("guild_id manquant"));
        }
        let enabled = self.control.is_enabled(&guild_id).await.map_err(|error| {
            tracing::error!(%error, "Lecture de l'etat Atrium impossible");
            Status::internal("lecture de l'etat impossible")
        })?;
        Ok(Response::new(proto::BotStateResponse { enabled }))
    }

    async fn set_state(
        &self,
        request: Request<proto::SetBotStateRequest>,
    ) -> Result<Response<proto::BotStateResponse>, Status> {
        let input = request.into_inner();
        if input.guild_id.is_empty() || input.actor_id.is_empty() {
            return Err(Status::invalid_argument("guild_id ou actor_id manquant"));
        }
        self.control
            .set_enabled(&input.guild_id, input.enabled, &input.actor_id)
            .await
            .map_err(|error| {
                tracing::error!(%error, "Modification de l'etat Atrium impossible");
                Status::internal("modification de l'etat impossible")
            })?;
        tracing::info!(guild_id = %input.guild_id, actor_id = %input.actor_id, enabled = input.enabled, "Etat Atrium modifie");
        Ok(Response::new(proto::BotStateResponse {
            enabled: input.enabled,
        }))
    }
}

#[tonic::async_trait]
impl proto::rag_service_server::RagService for RagGrpc {
    async fn search_knowledge(
        &self,
        request: Request<proto::SearchKnowledgeRequest>,
    ) -> Result<Response<proto::SearchKnowledgeResponse>, Status> {
        let req = request.into_inner();
        let rag = self
            .rag
            .as_ref()
            .ok_or_else(|| Status::unavailable("RAG non configuré"))?;
        let chunks = rag
            .search_chunks(&req.guild_id, &req.query, req.limit)
            .await
            .map_err(|e| Status::internal(e))?;

        let proto_chunks = chunks
            .into_iter()
            .map(
                |(source, content, similarity)| proto::SearchKnowledgeChunk {
                    source,
                    content,
                    similarity,
                },
            )
            .collect();

        Ok(Response::new(proto::SearchKnowledgeResponse {
            chunks: proto_chunks,
        }))
    }
}
