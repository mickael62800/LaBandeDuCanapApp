use super::*;
use async_trait::async_trait;
use platform_core::sentinel::application::system::export_service::ExecuteExportUseCase;
use platform_core::sentinel::application::system::export_service::ExportResult;
use platform_core::sentinel::domain::errors::DomainError;
use std::sync::Arc;
use std::sync::Mutex;

struct MockExportUc {
    calls: Mutex<Vec<(String, String, String, i64)>>,
    result: Mutex<Result<ExportResult, DomainError>>,
}

impl MockExportUc {
    fn ok(data: &str, count: usize) -> Self {
        Self {
            calls: Mutex::new(vec![]),
            result: Mutex::new(Ok(ExportResult {
                data: data.into(),
                row_count: count,
            })),
        }
    }
    fn err(err: DomainError) -> Self {
        Self {
            calls: Mutex::new(vec![]),
            result: Mutex::new(Err(err)),
        }
    }
}

#[async_trait]
impl ExecuteExportUseCase for MockExportUc {
    async fn execute(
        &self,
        g: &str,
        j: &str,
        f: &str,
        max: i64,
    ) -> Result<ExportResult, DomainError> {
        self.calls
            .lock()
            .unwrap()
            .push((g.into(), j.into(), f.into(), max));
        match &*self.result.lock().unwrap() {
            Ok(r) => Ok(ExportResult {
                data: r.data.clone(),
                row_count: r.row_count,
            }),
            Err(DomainError::ValidationError(msg)) => {
                Err(DomainError::ValidationError(msg.clone()))
            }
            Err(DomainError::NotFound(msg)) => Err(DomainError::NotFound(msg.clone())),
            Err(e) => Err(DomainError::Internal(format!("{e:?}"))),
        }
    }
}

fn make_req(guild_id: &str, job_type: &str) -> Request<proto::ExecuteExportRequest> {
    Request::new(proto::ExecuteExportRequest {
        guild_id: guild_id.into(),
        job_type: job_type.into(),
        format: "csv".into(),
        max_rows: 100,
        filters_json: String::new(),
    })
}

#[tokio::test]
async fn empty_guild_id_returns_invalid_argument() {
    let grpc = ExportGrpc {
        uc: Arc::new(MockExportUc::ok("", 0)),
    };
    let err = grpc
        .execute_export(make_req("", "infractions"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("guild_id"));
}

#[tokio::test]
async fn empty_job_type_returns_invalid_argument() {
    let grpc = ExportGrpc {
        uc: Arc::new(MockExportUc::ok("", 0)),
    };
    let err = grpc.execute_export(make_req("g1", "")).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("job_type"));
}

#[tokio::test]
async fn successful_export_returns_data_and_count() {
    let uc = Arc::new(MockExportUc::ok("a,b\n1,2\n3,4\n", 2));
    let grpc = ExportGrpc { uc: uc.clone() };
    let resp = grpc
        .execute_export(make_req("g1", "infractions"))
        .await
        .unwrap();
    let inner = resp.into_inner();
    assert_eq!(inner.data, "a,b\n1,2\n3,4\n");
    assert_eq!(inner.row_count, 2);

    let calls = uc.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "g1");
    assert_eq!(calls[0].1, "infractions");
    assert_eq!(calls[0].2, "csv");
    assert_eq!(calls[0].3, 100);
}

#[tokio::test]
async fn uc_error_is_mapped_via_domain_to_status() {
    let grpc = ExportGrpc {
        uc: Arc::new(MockExportUc::err(DomainError::ValidationError(
            "bad format".into(),
        ))),
    };
    let err = grpc
        .execute_export(make_req("g1", "infractions"))
        .await
        .unwrap_err();
    // domain_to_status mappe ValidationError → InvalidArgument
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}
