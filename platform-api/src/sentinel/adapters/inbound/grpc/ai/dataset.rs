//! gRPC AiDataset — collecte des messages texte pour l'entrainement IA.
//!
//! Remplace `POST /api/ai-dataset/collect`, chemin le plus chaud du bot
//! (un message par message non-bot des guilds ou le module est actif).
//! L'ingestion passe par le use case dataset (validation + repository) —
//! plus de SQL direct dans l'inbound.
//!
//! Client-streaming : le bot maintient une stream longue duree, le serveur
//! insere chaque message au fil de l'eau. Un insert qui echoue est logge
//! mais n'interrompt PAS la stream (best-effort).

use std::sync::Arc;

use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::Streaming;
use tracing::warn;

use platform_core::sentinel::ports::inbound::ai::manage_dataset::ManageDatasetUseCase;
use platform_core::sentinel::ports::outbound::ai::dataset_repository::NewDatasetMessage;
use platform_proto::sentinel::ai_dataset::v1 as proto;
use platform_proto::sentinel::ai_dataset::v1::ai_dataset_service_server::AiDatasetService;
use platform_proto::sentinel::common::v1::Empty;

pub struct AiDatasetGrpc {
    pub dataset_uc: Arc<dyn ManageDatasetUseCase>,
}

#[tonic::async_trait]
impl AiDatasetService for AiDatasetGrpc {
    async fn collect_messages(
        &self,
        request: Request<Streaming<proto::CollectMessageRequest>>,
    ) -> Result<Response<Empty>, Status> {
        let mut stream = request.into_inner();

        while let Some(dto) = stream.message().await? {
            // Best-effort : un message invalide (validation) ou un insert rate
            // ne doit pas tuer la stream : on logge (insert) / ignore
            // (validation) et on continue.
            let result = self
                .dataset_uc
                .collect_message(NewDatasetMessage {
                    guild_id: dto.guild_id,
                    channel_id: dto.channel_id,
                    channel_name: dto.channel_name,
                    user_id: dto.user_id,
                    content: dto.content,
                })
                .await;
            if let Err(e) = result {
                match e {
                    platform_core::sentinel::domain::errors::DomainError::ValidationError(_) => {}
                    other => {
                        warn!(error = %other, "Echec insert ai_dataset (stream), message ignore");
                    }
                }
            }
        }

        Ok(Response::new(Empty {}))
    }
}
