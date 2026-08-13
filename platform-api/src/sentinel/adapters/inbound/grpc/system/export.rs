//! gRPC handler pour le service d'export.

use std::sync::Arc;

use platform_proto::sentinel::export::v1 as proto;
use platform_proto::sentinel::export::v1::export_service_server::ExportService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use platform_core::sentinel::application::system::export_service::ExecuteExportUseCase;

pub struct ExportGrpc {
    pub uc: Arc<dyn ExecuteExportUseCase>,
}

#[tonic::async_trait]
impl ExportService for ExportGrpc {
    async fn execute_export(
        &self,
        request: Request<proto::ExecuteExportRequest>,
    ) -> Result<Response<proto::ExecuteExportResponse>, Status> {
        let req = request.into_inner();
        if req.guild_id.is_empty() {
            return Err(Status::invalid_argument("guild_id requis"));
        }
        if req.job_type.is_empty() {
            return Err(Status::invalid_argument("job_type requis"));
        }
        let result = self
            .uc
            .execute(&req.guild_id, &req.job_type, &req.format, req.max_rows)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ExecuteExportResponse {
            data: result.data,
            row_count: result.row_count as u64,
        }))
    }
}

#[cfg(test)]
#[path = "tests/export.rs"]
mod tests;
