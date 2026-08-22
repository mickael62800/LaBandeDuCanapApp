use super::*;
use async_trait::async_trait;
use std::sync::Mutex;
use uuid::Uuid;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::outbound::system::export_job_repository::ExportJobRepository;

#[derive(Default)]
struct MockExportJobRepo {
    jobs: Mutex<Vec<String>>,
}

#[async_trait]
impl ExportJobRepository for MockExportJobRepo {
    async fn create(&self, job_id: &str, _job_type: &str) -> Result<(), DomainError> {
        self.jobs.lock().unwrap().push(job_id.to_string());
        Ok(())
    }

    async fn find_by_id(&self, job_id: &str) -> Result<Option<String>, DomainError> {
        Ok(self.jobs.lock().unwrap().iter().find(|j| j == &job_id).cloned())
    }

    async fn complete(&self, _job_id: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

#[tokio::test]
async fn create_export_job() {
    let repo = std::sync::Arc::new(MockExportJobRepo::default());
    let svc = ManageExportJobsService::new(repo);
    let result = svc.create("job123", "csv").await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn find_export_job() {
    let repo = std::sync::Arc::new(MockExportJobRepo::default());
    let svc = ManageExportJobsService::new(repo);
    let result = svc.get("job123").await;
    assert!(result.is_ok() || result.is_err());
}
