//! Adaptateur gRPC unaire et streaming consommé par les microservices & bots Nexus.

use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use platform_core::nexus::ports::inbound::game::manage_game_servers::ManageGameServersUseCase;
use platform_proto::nexus::game::v1::{
    self as proto,
    game_server_service_server::{GameServerService, GameServerServiceServer},
};

use crate::nexus::bootstrap::AppState;

#[allow(dead_code)]
pub async fn serve(state: AppState) {
    let port: u16 = std::env::var("NEXUS_GRPC_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3101);

    let addr = format!("0.0.0.0:{port}").parse().expect("gRPC address");
    let service = NexusGrpcService {
        game_servers_uc: state.game_servers_uc.clone(),
    };

    tracing::info!(%addr, "nexus-api gRPC démarré sur port {port}");
    tonic::transport::Server::builder()
        .add_service(GameServerServiceServer::new(service))
        .serve(addr)
        .await
        .expect("serveur gRPC Nexus");
}

#[allow(dead_code)]
pub struct NexusGrpcService {
    pub game_servers_uc: Arc<dyn ManageGameServersUseCase>,
}

#[tonic::async_trait]
impl GameServerService for NexusGrpcService {
    type StreamLogsStream =
        Pin<Box<dyn Stream<Item = Result<proto::LogChunk, Status>> + Send + 'static>>;
    type StreamStatsStream =
        Pin<Box<dyn Stream<Item = Result<proto::ContainerStatsResponse, Status>> + Send + 'static>>;

    async fn stream_logs(
        &self,
        request: Request<proto::StreamLogsRequest>,
    ) -> Result<Response<Self::StreamLogsStream>, Status> {
        let req = request.into_inner();
        let server_id = req
            .server_id
            .parse::<Uuid>()
            .map_err(|_| Status::invalid_argument("server_id UUID invalide"))?;

        let logs = self
            .game_servers_uc
            .get_logs(server_id, req.tail_lines)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let server_id_str = req.server_id.clone();
        let output_stream = async_stream::try_stream! {
            for line in logs {
                yield proto::LogChunk {
                    server_id: server_id_str.clone(),
                    line,
                    timestamp_epoch_ms: chrono::Utc::now().timestamp_millis(),
                };
            }
        };

        Ok(Response::new(Box::pin(output_stream)))
    }

    async fn stream_stats(
        &self,
        request: Request<proto::StreamStatsRequest>,
    ) -> Result<Response<Self::StreamStatsStream>, Status> {
        let req = request.into_inner();
        let server_id = req
            .server_id
            .parse::<Uuid>()
            .map_err(|_| Status::invalid_argument("server_id UUID invalide"))?;

        let stats = self
            .game_servers_uc
            .get_stats(server_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let output_stream = async_stream::try_stream! {
            yield proto::ContainerStatsResponse {
                server_id: req.server_id,
                cpu_percentage: stats.cpu_percent,
                memory_used_bytes: stats.memory_used_bytes,
                memory_limit_bytes: stats.memory_limit_bytes,
                network_rx_bytes: stats.network_rx_bytes,
                network_tx_bytes: stats.network_tx_bytes,
            };
        };

        Ok(Response::new(Box::pin(output_stream)))
    }

    async fn execute_rcon(
        &self,
        request: Request<proto::ExecuteRconRequest>,
    ) -> Result<Response<proto::ExecuteRconResponse>, Status> {
        let req = request.into_inner();
        let server_id = req
            .server_id
            .parse::<Uuid>()
            .map_err(|_| Status::invalid_argument("server_id UUID invalide"))?;

        let response = self
            .game_servers_uc
            .execute_rcon(server_id, &req.command, "grpc-actor")
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::ExecuteRconResponse {
            server_id: req.server_id,
            response,
            success: true,
        }))
    }
}
