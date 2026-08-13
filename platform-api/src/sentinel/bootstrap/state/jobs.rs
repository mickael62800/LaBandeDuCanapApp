//! Capacites reservees aux endpoints de jobs internes.

use axum::extract::FromRef;
use std::sync::Arc;

use platform_core::sentinel::ports::inbound::system::run_internal_job::RunInternalJobUseCase;

use super::AppState;

#[derive(Clone)]
pub struct InternalJobsState {
    pub runner: Arc<dyn RunInternalJobUseCase>,
    pub(crate) job_lock_pool: sqlx::PgPool,
}

impl InternalJobsState {
    pub(crate) fn job_lock_pool(&self) -> sqlx::PgPool {
        self.job_lock_pool.clone()
    }
}

impl FromRef<AppState> for InternalJobsState {
    fn from_ref(state: &AppState) -> Self {
        state.jobs.clone()
    }
}
