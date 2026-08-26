//! Endpoints INTERNES utilises par le worker game-portal.
//!
//! Securite : seuls le Bearer global NEXUS_API_KEY protege ces endpoints
//! (le worker est un processus de confiance qui partage la meme cle).

use axum::extract::State;
use axum::Json;

use crate::nexus::adapters::inbound::http::handlers::ApiError;
use crate::nexus::bootstrap::AppState;

use platform_core::nexus::application::game::worker_jobs::{
    run_daily_ping, run_health_check, run_idle_shutdown, run_image_cleanup, run_purge_history,
    run_reconciler, run_reveal_ip, JobContext, JobReport,
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
    match crate::shared::job_lock::run(&state.job_pool, &format!("nexus:{name}"), || async {
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

/// Menage de l'historique de surveillance. La retention se regle par
/// l'environnement : c'est un parametre d'exploitation (place disque), pas un
/// choix de communaute.
pub async fn job_purge_history(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    let retention = std::env::var("GAME_PERF_HISTORY_RETENTION_DAYS")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(platform_core::nexus::application::game::worker_jobs::RETENTION_JOURS_DEFAUT);
    locked(&state, "purge-history", || async {
        run_purge_history(&ctx(&state), retention).await
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

/// Verification periodique des jeux mentionnables.
///
/// Le job ne repare RIEN : il demande a chaque guilde son inventaire Discord,
/// que le bot deposera ensuite. Le rapport de divergence s'en trouve rafraichi
/// tout seul, et une desynchronisation cesse d'attendre qu'un humain la
/// soupconne pour etre visible. Le sens de la reparation reste un choix humain
/// (cf. `game_sync_service`).
pub async fn job_mention_sync(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "mention-sync", || async {
        let service = platform_core::nexus::application::game_sync_service::GameSyncService::new(
            state.game_repo.clone(),
            state.game_sync_repo.clone(),
            state.events.clone(),
        );
        let guilds = state.game_sync_repo.guilds_with_games().await?;
        for guild_id in &guilds {
            service.request_inventory(guild_id).await;
        }
        Ok(JobReport {
            job: "mention_sync",
            processed: guilds.len(),
            errors: 0,
            details: serde_json::json!({ "guilds": guilds.len() }),
        })
    })
    .await
}

/// Ferme les defis de Coussin Piege restes sans reponse.
///
/// Lancer un defi pose immediatement le delai d'attente de l'attaquant. Tant
/// que l'adversaire ne repond ni oui ni non, l'attaquant reste donc puni d'une
/// bagarre qui n'a jamais eu lieu, et le defi traine indefiniment. Le job les
/// ferme passe leur echeance (24 h) et rend son tour a l'attaquant.
///
/// Rien n'est preleve a personne : un defi en attente n'a debite aucune mise,
/// et les paris ne s'ouvrent qu'une fois le defi accepte.
pub async fn job_coussin_expire_combats(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "coussin-expire-combats", || async {
        let expired = state.coussin_repo.expire_pending_combats().await?;
        for combat in &expired {
            tracing::info!(
                combat_id = %combat.id,
                guild_id = %combat.guild_id,
                mise = combat.mise,
                "defi Coussin ferme faute de reponse"
            );
        }
        Ok(JobReport {
            job: "coussin_expire_combats",
            processed: expired.len(),
            errors: 0,
            details: serde_json::json!({ "expired": expired.len() }),
        })
    })
    .await
}

/// Resout les fouilles dont la fenetre de defense s'est fermee sans reaction.
///
/// C'est ce passage qui donne son sens au bouton : ne pas reagir n'est pas
/// « il ne se passe rien », c'est une reponse — celle qui coute son malus a la
/// victime et laisse passer le voleur beaucoup plus facilement.
///
/// Le denouement est publie sur le bus pour que le bot puisse le raconter dans
/// le salon d'origine, meme s'il a redemarre entre-temps.
pub async fn job_coussin_expire_steals(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "coussin-expire-steals", || async {
        let outcomes = state.coussin_steal.resolve_expired(100).await?;
        for outcome in &outcomes {
            state
                .events
                .publish(
                    platform_core::nexus::ports::outbound::events::coussin_events::STEAL_RESOLVED,
                    crate::nexus::adapters::inbound::http::handlers::coussin::steal_outcome_json(
                        outcome,
                    ),
                )
                .await;
        }
        Ok(JobReport {
            job: "coussin_expire_steals",
            processed: outcomes.len(),
            errors: 0,
            details: serde_json::json!({ "resolved": outcomes.len() }),
        })
    })
    .await
}

/// Surveillance des serveurs de jeu : seuils depasses -> webhook Discord.
///
/// Cote serveur, et non plus dans le navigateur : une alerte qui ne veille que
/// lorsqu'on regarde la page ne sert a rien, c'est la nuit qu'un serveur
/// sature.
pub async fn job_game_alerts(State(state): State<AppState>) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "game-alerts", || async {
        let rapport = crate::nexus::jobs::game_alerts::run(&state).await?;
        Ok(JobReport {
            job: "game_alerts",
            processed: rapport.sent,
            errors: rapport.errors,
            details: serde_json::json!({
                "surveilles": rapport.checked,
                "envoyees": rapport.sent,
            }),
        })
    })
    .await
}

/// Pilotage des serveurs dans le temps : plages d'ouverture pour les uns,
/// redemarrages periodiques pour les autres.
///
/// Passage court : une plage se termine a la minute pres, et l'annonce
/// « redemarrage dans 1 minute » ne veut plus rien dire avec deux minutes de
/// retard.
pub async fn job_game_schedules(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "game-schedules", || async {
        let rapport = crate::nexus::jobs::game_schedules::run(&state).await?;
        Ok(JobReport {
            job: "game_schedules",
            processed: rapport.started + rapport.stopped + rapport.warned + rapport.restarted,
            errors: rapport.errors,
            details: serde_json::json!({
                "ouverts": rapport.started,
                "fermes": rapport.stopped,
                "preavis": rapport.warned,
                "redemarres": rapport.restarted,
            }),
        })
    })
    .await
}

/// POST /api/games/internal/jobs/session-announcements
pub async fn job_session_announcements(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "session-announcements", || async {
        let rapport = crate::nexus::jobs::session_announcements::run(&state).await?;
        Ok(JobReport {
            job: "session_announcements",
            processed: rapport.relancees + rapport.abandons,
            errors: rapport.errors,
            details: serde_json::json!({
                "relancees": rapport.relancees,
                "abandons": rapport.abandons,
            }),
        })
    })
    .await
}
