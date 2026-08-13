//! Capacites reservees aux endpoints de jobs internes.

use axum::extract::FromRef;

use super::AppState;

#[derive(Clone)]
pub struct InternalJobsState {
    pub pg_pool: sqlx::PgPool,
    pub redis_client: redis::Client,
}

impl FromRef<AppState> for InternalJobsState {
    fn from_ref(state: &AppState) -> Self {
        state.jobs.clone()
    }
}
