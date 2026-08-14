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

// ── Adaptateur de presence Palworld ──────────────────────────────────────

/// Hote RCON : les conteneurs de jeu publient leur port sur l'hote, que
/// `platform-api` joint par la boucle locale (meme convention que les jobs de
/// `worker_jobs`).
const RCON_HOST: &str = "127.0.0.1";

/// Haut fait attribue a la premiere presence constatee sur un serveur Palworld.
const PALWORLD_FIRST_LAUNCH: &str = "first_launch_palworld";
/// Haut fait attribue quand beaucoup de joueurs sont connectes en meme temps.
const PALWORLD_MASSIVE_SESSION: &str = "palworld_massive_session";

/// POST /api/games/internal/jobs/palworld-presence
///
/// Seul adaptateur d'evenements Palworld en place. Il s'appuie sur RCON
/// `ShowPlayers`, qui renvoie le **SteamID64** de chaque joueur connecte :
/// c'est ce qui rend la presence VERIFIABLE et reliable a un membre Discord
/// par `game_player_links`. Les hauts faits qui demandent un fait de jeu
/// (boss, elevage, base) ne sont pas observables par ce canal et restent en
/// attribution manuelle.
pub async fn job_palworld_presence(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    locked(&state, "palworld-presence", || async {
        run_palworld_presence(&state).await
    })
    .await
}

async fn run_palworld_presence(
    state: &AppState,
) -> Result<JobReport, platform_core::nexus::domain::errors::DomainError> {
    use platform_core::nexus::domain::entities::game::presence;
    use platform_core::nexus::ports::inbound::achievements::GameUnlockCommand;
    use platform_core::nexus::ports::outbound::game::rcon_client::RconConnectionParams;

    let servers = state.game_server_repo.list_running().await?;
    let mut processed = 0usize;
    let mut errors = 0usize;
    let mut unlocked = 0usize;

    for server in &servers {
        // Ne concerne que Palworld : les autres jeux n'exposent pas d'identite
        // verifiable par ce canal.
        let slug = state
            .game_template_repo
            .find_by_id(server.template_id)
            .await
            .ok()
            .flatten()
            .map(|t| t.slug)
            .unwrap_or_default();
        if !slug.to_ascii_lowercase().starts_with("palworld") {
            continue;
        }
        let (Some(port), Some(password)) = (server.rcon_port, server.rcon_password.clone()) else {
            continue;
        };
        let cfg = platform_core::nexus::application::game::config_loader::load_game_portal_config(
            &state.bot_config_repo,
            &server.guild_id,
        )
        .await?;
        if !cfg.rcon_enabled {
            continue;
        }
        processed += 1;

        let params = RconConnectionParams {
            host: RCON_HOST.to_string(),
            port,
            password,
            timeout_secs: cfg.rcon_timeout_secs,
        };
        let raw = match state
            .game_rcon_client
            .execute(&params, presence::players_command(&slug))
            .await
        {
            Ok(resp) => resp.raw,
            Err(e) => {
                // Un serveur qui finit de demarrer refuse encore RCON : ce
                // n'est pas une anomalie, le prochain passage reessaiera.
                tracing::debug!(error = %e, server_id = %server.id, "presence Palworld : RCON indisponible");
                errors += 1;
                continue;
            }
        };

        // Seuls les joueurs dont le serveur expose une identite Steam sont
        // exploitables ; les autres sont comptes mais ne debloquent rien.
        let identifies: Vec<String> = presence::parse_players(&slug, &raw)
            .into_iter()
            .filter_map(|p| p.game_player_id)
            .collect();
        if identifies.is_empty() {
            continue;
        }

        // Seuil du haut fait « grande expedition », lu dans sa definition :
        // le document impose que ces valeurs soient configurables et jamais
        // codees en dur.
        let seuil_massive = state
            .achievements_uc
            .list_definitions(Some("palworld"))
            .await
            .ok()
            .and_then(|defs| {
                defs.into_iter()
                    .find(|d| d.code == PALWORLD_MASSIVE_SESSION)
                    .and_then(|d| d.criteria.get("players").and_then(|v| v.as_u64()))
            })
            .unwrap_or(8) as usize;

        for steam_id in &identifies {
            // `source_event_id` STABLE par (guilde, joueur, haut fait) : rejouer
            // le job ne cree pas de doublon, l'unicite en base absorbe le
            // second passage.
            let mut demandes = vec![(
                PALWORLD_FIRST_LAUNCH,
                format!("palworld:first_launch:{}:{}", server.guild_id, steam_id),
            )];
            if identifies.len() >= seuil_massive {
                demandes.push((
                    PALWORLD_MASSIVE_SESSION,
                    format!("palworld:massive:{}:{}", server.guild_id, steam_id),
                ));
            }

            for (code, source_event_id) in demandes {
                match state
                    .achievements_uc
                    .unlock_from_game_event(GameUnlockCommand {
                        guild_id: server.guild_id.clone(),
                        game: "palworld".to_string(),
                        game_player_id: steam_id.clone(),
                        achievement_code: code.to_string(),
                        source_event_id,
                    })
                    .await
                {
                    Ok(outcome) => {
                        if super::super::achievements::publish_unlock(state, &outcome).await {
                            unlocked += 1;
                        }
                    }
                    // Joueur sans liaison verifiee : cas NOMINAL et frequent
                    // (le membre n'a pas encore fait `/haut-faits lier`). On ne
                    // le compte pas comme une erreur.
                    Err(platform_core::nexus::domain::errors::DomainError::NotFound(_)) => {}
                    Err(e) => {
                        tracing::warn!(error = %e, code, "presence Palworld : attribution echouee");
                        errors += 1;
                    }
                }
            }
        }
    }

    Ok(JobReport {
        job: "palworld_presence",
        processed,
        errors,
        details: serde_json::json!({ "unlocked": unlocked }),
    })
}
