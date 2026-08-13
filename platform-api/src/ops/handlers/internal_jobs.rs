use axum::{extract::State, Json};

use crate::ops::{jobs::alerts_dispatcher, ApiError, AppState};

pub async fn dispatch_alerts(
    State(state): State<AppState>,
) -> Result<Json<alerts_dispatcher::DispatchReport>, ApiError> {
    let report = crate::shared::job_lock::run(&state.pg_pool, "ops:dispatch-alerts", || {
        alerts_dispatcher::run(
            &state.pg_pool,
            &state.redis_client,
            &state.config.security_alerts_webhook,
        )
    })
    .await
    .map_err(|error| ApiError(axum::http::StatusCode::INTERNAL_SERVER_ERROR, error))?;
    report
        .map(Json)
        .ok_or_else(|| ApiError(axum::http::StatusCode::CONFLICT, "job deja actif".into()))
}
