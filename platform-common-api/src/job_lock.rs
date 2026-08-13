use std::future::Future;
use axum::{extract::{Request, State}, http::StatusCode, middleware::Next, response::{IntoResponse, Response}, Json};

pub async fn run<T, F, Fut>(pool: &sqlx::PgPool, job: &str, operation: F) -> Result<Option<T>, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let mut connection = pool.acquire().await.map_err(|error| format!("job lock acquire: {error}"))?;
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtext($1))")
        .bind(job).fetch_one(&mut *connection).await.map_err(|error| format!("job lock: {error}"))?;
    if !acquired { return Ok(None); }
    let result = operation().await;
    if let Err(error) = sqlx::query_scalar::<_, bool>("SELECT pg_advisory_unlock(hashtext($1))")
        .bind(job).fetch_one(&mut *connection).await {
        tracing::error!(job, %error, "liberation du verrou impossible");
    }
    result.map(Some)
}

pub async fn middleware(State(pool): State<sqlx::PgPool>, request: Request, next: Next) -> Response {
    let Some(job) = request.headers().get("x-scheduler-job").and_then(|v| v.to_str().ok()).map(str::to_owned) else { return next.run(request).await; };
    let mut connection = match pool.acquire().await { Ok(c) => c, Err(error) => return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": error.to_string()}))).into_response() };
    let acquired = sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock(hashtext($1))").bind(&job).fetch_one(&mut *connection).await.unwrap_or(false);
    if !acquired { return (StatusCode::ACCEPTED, Json(serde_json::json!({"job": job, "locked": true}))).into_response(); }
    let response = next.run(request).await;
    let _ = sqlx::query_scalar::<_, bool>("SELECT pg_advisory_unlock(hashtext($1))").bind(&job).fetch_one(&mut *connection).await;
    response
}
