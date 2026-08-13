use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use platform_core::sentinel::ports::inbound::system::run_internal_job::InternalJobOutcome;
use serde_json::json;

use crate::sentinel::bootstrap::state::InternalJobsState;

pub async fn run(
    State(state): State<InternalJobsState>,
    Path(job): Path<String>,
) -> impl IntoResponse {
    match state.runner.run(&job).await {
        Ok(InternalJobOutcome::Executed) => (
            StatusCode::OK,
            Json(json!({"job": job, "processed": 1, "errors": 0})),
        ),
        Ok(InternalJobOutcome::Locked) => (
            StatusCode::ACCEPTED,
            Json(json!({"job": job, "processed": 0, "errors": 0, "locked": true})),
        ),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"job": job, "processed": 0, "errors": 1, "error": error})),
        ),
    }
}
