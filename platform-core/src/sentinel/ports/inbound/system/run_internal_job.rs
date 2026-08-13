use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InternalJobOutcome {
    Executed,
    Locked,
}

#[async_trait]
pub trait RunInternalJobUseCase: Send + Sync {
    async fn run(&self, job: &str) -> Result<InternalJobOutcome, String>;
}
