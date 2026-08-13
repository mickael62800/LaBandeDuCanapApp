//! Endpoints INTERNES utilises par le worker game-portal.
//!
//! Securite : seuls le Bearer global NEXUS_API_KEY protege ces endpoints
//! (le worker est un processus de confiance qui partage la meme cle).

use axum::extract::State;
use axum::Json;

use crate::nexus::adapters::inbound::http::handlers::ApiError;
use crate::nexus::bootstrap::AppState;

use platform_core::nexus::application::game::worker_jobs::{
    run_daily_ping, run_health_check, run_idle_shutdown, run_image_cleanup, run_reconciler,
    run_reveal_ip, JobContext, JobReport,
};

async fn locked<F, Fut>(
    state: &AppState,
    name: &str,
    operation: F,
) -> Result<Json<JobReport>, ApiError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<
        Output = Result<JobReport, platform_core::nexus::domain::errors::DomainError>,
    >,
{
    match platform_common_api::job_lock::run(&state.job_pool, &format!("nexus:{name}"), || async {
        operation().await.map_err(|error| error.to_string())
    })
    .await
    {
        Ok(Some(report)) => Ok(Json(report)),
        Ok(None) => Err(platform_core::nexus::domain::errors::DomainError::Conflict(
            "job deja actif".into(),
        )
        .into()),
        Err(error) => {
            Err(platform_core::nexus::domain::errors::DomainError::Infrastructure(error).into())
        }
    }
}

fn ctx(state: &AppState) -> JobContext {
    JobContext {
        server_repo: state.game_server_repo.clone(),
        template_repo: state.game_template_repo.clone(),
        audit_repo: state.game_audit_repo.clone(),
        session_repo: state.game_session_repo.clone(),
        container_runtime: state.game_container_runtime.clone(),
        rcon_client: state.game_rcon_client.clone(),
        port_allocator: state.game_port_allocator.clone(),
        bot_config: state.bot_config_repo.clone(),
        events: state.events.clone(),
    }
}

pub async fn job_health_check(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "health-check", || async {
        run_health_check(&ctx(&state)).await
    })
    .await
}

pub async fn job_idle_shutdown(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "idle-shutdown", || async {
        run_idle_shutdown(&ctx(&state)).await
    })
    .await
}

pub async fn job_reconcile(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "reconcile", || async {
        run_reconciler(&ctx(&state)).await
    })
    .await
}

pub async fn job_image_cleanup(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "image-cleanup", || async {
        run_image_cleanup(&ctx(&state)).await
    })
    .await
}

pub async fn job_reveal_ip(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "reveal-ip", || async {
        run_reveal_ip(&ctx(&state)).await
    })
    .await
}

pub async fn job_daily_ping(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "daily-ping", || async {
        run_daily_ping(&ctx(&state)).await
    })
    .await
}

/// Auto-start des serveurs programmes : demarre le conteneur des serveurs
/// `scheduled` dont l'ouverture est a moins de `PREP_LEAD_MINUTES`. Contrairement
/// aux autres jobs, il passe par le use case complet `start()` (allocation ports,
/// creation + demarrage du conteneur) — d'ou l'implementation ici, au niveau API,
/// plutot que dans `worker_jobs` qui n'a pas acces au use case. Les salons Discord
/// existent deja (crees a la programmation), on ne republie donc aucun evenement.
pub async fn job_auto_start(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "auto-start", || async {
        run_auto_start(&state).await
    })
    .await
}

async fn run_auto_start(
    state: &AppState,
) -> Result<JobReport, platform_core::nexus::domain::errors::DomainError> {
    let due = state.game_server_repo.list_scheduled_due_to_start().await?;
    let mut processed = 0usize;
    let mut errors = 0usize;
    for server in &due {
        match state.game_servers_uc.start(server.id, "system").await {
            Ok(()) => processed += 1,
            Err(e) => {
                tracing::warn!(error = %e, server_id = %server.id, "auto-start: echec demarrage serveur programme");
                errors += 1;
            }
        }
    }
    Ok(JobReport {
        job: "auto_start",
        processed,
        errors,
        details: serde_json::json!({ "due": due.len() }),
    })
}
