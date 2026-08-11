//! Logique des 3 jobs du game-portal-worker, exposees via l'API.
//!
//! Ces fonctions sont appelees par les endpoints internes /api/games/internal/jobs/*
//! que le worker invoque sur un timer. Elles utilisent les use cases existants
//! et les ports outbound pour ne pas dupliquer la logique.

use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::application::game::config_loader::load_game_portal_config;
use crate::domain::entities::game::audit::GameAuditAction;
use crate::domain::entities::game::server::{should_auto_restart, GameServerStatus};
use crate::domain::errors::DomainError;
use crate::ports::outbound::game::container_runtime::{ContainerRuntime, ContainerState};
use crate::ports::outbound::game::game_audit_repository::GameAuditRepository;
use crate::ports::outbound::game::game_server_repository::{
    GameServerRepository, GameServerRuntimeUpdate,
};
use crate::ports::outbound::game::player_session_repository::PlayerSessionRepository;
use crate::ports::outbound::game::port_allocator::{PortAllocator, PortKind};
use crate::ports::outbound::game::rcon_client::{RconClient, RconConnectionParams};
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

const RCON_HOST: &str = "127.0.0.1";

/// Bag d'adapters pour les jobs (evite des signatures kilometriques).
pub struct JobContext {
    pub server_repo: Arc<dyn GameServerRepository>,
    pub template_repo:
        Arc<dyn crate::ports::outbound::game::game_template_repository::GameTemplateRepository>,
    pub audit_repo: Arc<dyn GameAuditRepository>,
    pub session_repo: Arc<dyn PlayerSessionRepository>,
    pub container_runtime: Arc<dyn ContainerRuntime>,
    pub rcon_client: Arc<dyn RconClient>,
    pub port_allocator: Arc<dyn PortAllocator>,
    pub bot_config: Arc<dyn BotConfigRepository>,
    pub events: Arc<dyn crate::ports::outbound::events::EventPublisher>,
}

/// Stats retournees par chaque job (pour observabilite worker -> log API).
#[derive(Debug, serde::Serialize)]
pub struct JobReport {
    pub job: &'static str,
    pub processed: usize,
    pub errors: usize,
    pub details: serde_json::Value,
}

// ════════════════════════════════════════════════════════════════════════
// JOB 1 : HEALTH CHECK
// ════════════════════════════════════════════════════════════════════════

/// Pour chaque serveur `running`, query player count via RCON `list`. Met
/// a jour last_player_count + last_active_at, ouvre/ferme les sessions.
pub async fn run_health_check(ctx: &JobContext) -> Result<JobReport, DomainError> {
    let servers = ctx.server_repo.list_running().await?;
    let mut errors = 0usize;
    let mut details = serde_json::Map::new();

    for server in &servers {
        let cfg = load_game_portal_config(&ctx.bot_config, &server.guild_id).await?;
        if !cfg.rcon_enabled || server.rcon_port.is_none() || server.rcon_password.is_none() {
            // Pour les serveurs sans RCON (comme Valheim), rafraîchir l'activité quand le serveur est actif
            // pour éviter une extinction par erreur.
            let _ = ctx.server_repo.update_player_activity(server.id, 0).await;
            continue;
        }
        let port = server.rcon_port.unwrap();
        let pwd = server.rcon_password.clone().unwrap();
        let params = RconConnectionParams {
            host: RCON_HOST.to_string(),
            port,
            password: pwd,
            timeout_secs: cfg.rcon_timeout_secs,
        };
        let resp = match ctx.rcon_client.execute(&params, "list").await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, server_id = %server.id, "health rcon failed");
                errors += 1;
                continue;
            }
        };
        let (count, players) = parse_minecraft_list(&resp.raw);

        // Maj last_player_count + last_active_at si > 0
        if let Err(e) = ctx
            .server_repo
            .update_player_activity(server.id, count)
            .await
        {
            warn!(error = %e, "update_player_activity");
            errors += 1;
        }

        // Diff sessions actives <-> liste actuelle
        let active = ctx.session_repo.list_active(server.id).await?;
        let active_names: std::collections::HashSet<String> =
            active.iter().map(|s| s.player_name.clone()).collect();
        let new_names: std::collections::HashSet<String> = players.iter().cloned().collect();

        for joined in new_names.difference(&active_names) {
            if let Err(e) = ctx.session_repo.open(server.id, joined).await {
                warn!(error = %e, "open session");
                errors += 1;
            }
        }
        for left in active_names.difference(&new_names) {
            if let Err(e) = ctx.session_repo.close(server.id, left).await {
                warn!(error = %e, "close session");
                errors += 1;
            }
        }
        // Observation saine (RCON repond) : si le serveur avait des
        // tentatives de redemarrage en cours, on remet le compteur a 0 pour
        // qu'un futur crash reparte d'un backoff propre. Cheap : seulement si
        // restart_attempts > 0.
        if server.restart_attempts > 0 {
            if let Err(e) = ctx.server_repo.reset_restart_attempts(server.id).await {
                warn!(error = %e, server_id = %server.id, "reset_restart_attempts");
            }
        }

        details.insert(server.id.to_string(), serde_json::json!(count));
    }

    Ok(JobReport {
        job: "health_check",
        processed: servers.len(),
        errors,
        details: serde_json::Value::Object(details),
    })
}

/// Parse la sortie de la commande `list` Minecraft :
/// `There are 2 of a max of 20 players online: alice, bob`
fn parse_minecraft_list(raw: &str) -> (i32, Vec<String>) {
    // Compte
    let count = raw
        .split(' ')
        .find_map(|w| w.parse::<i32>().ok())
        .unwrap_or(0);
    // Liste apres ":"
    let players: Vec<String> = if let Some(idx) = raw.find(':') {
        raw[idx + 1..]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        vec![]
    };
    (count, players)
}

// ════════════════════════════════════════════════════════════════════════
// JOB 2 : IDLE SHUTDOWN
// ════════════════════════════════════════════════════════════════════════

/// Stop les serveurs running dont `last_active_at` est anterieur a
/// `idle_shutdown_days` jours (override par instance ou template).
pub async fn run_idle_shutdown(ctx: &JobContext) -> Result<JobReport, DomainError> {
    let servers = ctx.server_repo.list_running().await?;
    let mut stopped = 0usize;
    let mut errors = 0usize;

    let now = chrono::Utc::now();
    for server in &servers {
        // Resoud le seuil idle (instance override -> template default).
        let cfg = load_game_portal_config(&ctx.bot_config, &server.guild_id).await?;
        let days = server
            .idle_shutdown_days
            .unwrap_or(cfg.default_idle_shutdown_days);
        if days <= 0 {
            continue;
        }
        // Garde-fou : on ne coupe JAMAIS un serveur pour lequel on n'a pas de
        // signal de presence fiable. last_player_count / last_active_at ne
        // sont alimentes que via RCON (job health_check). Pour un serveur sans
        // RCON (config off, ou pas de port/password alloue) le compteur reste
        // a 0 alors qu'il peut etre plein -> l'arreter serait une coupure a
        // tort. Mieux vaut le laisser tourner que de tuer un serveur peuple.
        let has_player_signal =
            cfg.rcon_enabled && server.rcon_port.is_some() && server.rcon_password.is_some();
        if !has_player_signal {
            continue;
        }
        let cutoff = now - chrono::Duration::days(days as i64);
        let last = server.last_active_at.unwrap_or(server.created_at);
        if last >= cutoff {
            continue;
        }
        if server.last_player_count > 0 {
            // Quelqu'un est connecte malgre last_active_at vieux : skip.
            continue;
        }

        info!(server_id = %server.id, days, "idle shutdown");

        // Stop via container_runtime direct (pas de RBAC : c'est le worker).
        if let Some(cid) = &server.container_id {
            if let Err(e) = ctx.container_runtime.stop_container(cid, 30).await {
                warn!(error = %e, "stop container failed");
                errors += 1;
                continue;
            }
        }
        if let Err(e) = ctx
            .server_repo
            .update_runtime(
                server.id,
                GameServerRuntimeUpdate {
                    status: Some(GameServerStatus::Stopped),
                    stopped_at_now: true,
                    ..Default::default()
                },
            )
            .await
        {
            warn!(error = %e, "update_runtime stopped");
            errors += 1;
        }
        let _ = ctx.session_repo.close_all_active(server.id).await;
        let _ = ctx
            .audit_repo
            .log(
                &server.guild_id,
                Some(server.id),
                None, // actor = system
                GameAuditAction::IdleShutdown,
                serde_json::json!({ "idle_days": days }),
            )
            .await;
        stopped += 1;
    }

    Ok(JobReport {
        job: "idle_shutdown",
        processed: stopped,
        errors,
        details: serde_json::json!({ "stopped": stopped }),
    })
}

// ════════════════════════════════════════════════════════════════════════
// JOB 3 : RECONCILER
// ════════════════════════════════════════════════════════════════════════

/// Reconcilie l'etat DB <-> Docker reel.
///   - Containers Docker avec label sentinel.managed=game-portal mais
///     pas de ligne game_servers correspondante : log warning (orphelins).
///   - Lignes game_servers avec status running mais container disparu :
///     marque error + libere les ports.
pub async fn run_reconciler(ctx: &JobContext) -> Result<JobReport, DomainError> {
    let active_servers = ctx.server_repo.list_active().await?;
    let docker_containers = ctx.container_runtime.list_managed_containers().await?;

    let mut details = serde_json::Map::new();
    let mut errors = 0usize;
    let now = chrono::Utc::now();

    // Index des conteneurs par identifiant de serveur.
    //
    // Les deux generations de label sont acceptees : `nexus.server_id` est le
    // nom canonique, `sentinel.server_id` celui que porte la flotte creee
    // avant le renommage. Ne lire que le nouveau ferait passer tous les
    // serveurs deja en service pour des orphelins — et le reconciler les
    // arreterait.
    let docker_by_id: std::collections::HashMap<String, &_> = docker_containers
        .iter()
        .filter_map(|c| {
            c.labels
                .get("nexus.server_id")
                .or_else(|| c.labels.get("sentinel.server_id"))
                .map(|sid| (sid.clone(), c))
        })
        .collect();

    // Helper local : libere les ports d'un serveur + log un crash audit.
    async fn mark_crashed(ctx: &JobContext, s: &crate::domain::entities::game::server::GameServer) {
        if let Some(p) = s.host_port {
            let _ = ctx.port_allocator.release(PortKind::Game, p).await;
        }
        if let Some(p) = s.rcon_port {
            let _ = ctx.port_allocator.release(PortKind::Rcon, p).await;
        }
        let _ = ctx
            .audit_repo
            .log(
                &s.guild_id,
                Some(s.id),
                None,
                GameAuditAction::CrashDetected,
                serde_json::json!({}),
            )
            .await;
    }

    // Helper local : gere le crash d'un serveur dont l'etat DESIRE est Running
    // (status Running) mais dont le container a exited. Tente un auto-restart
    // borne + backoff, ou abandonne (Error) une fois le plafond atteint.
    // Retourne `true` si une action "erreur/abandon" a ete comptee.
    async fn handle_running_crash(
        ctx: &JobContext,
        s: &crate::domain::entities::game::server::GameServer,
        now: chrono::DateTime<chrono::Utc>,
        max_attempts: i32,
        auto_restart_on_crash: bool,
        backoff_base_secs: i64,
        backoff_cap_secs: i64,
    ) -> bool {
        // Auto-restart desactive OU plafond atteint : on abandonne
        // definitivement (pas de crash loop, ou respect de la config).
        if !auto_restart_on_crash || s.restart_attempts >= max_attempts {
            let reason = if !auto_restart_on_crash {
                "crash : auto-restart desactive (reconciler)"
            } else {
                "crash : plafond de redemarrages atteint (reconciler)"
            };
            warn!(
                server_id = %s.id,
                attempts = s.restart_attempts,
                auto_restart_on_crash,
                "crash non redemarre, marque error"
            );
            let _ = ctx
                .server_repo
                .update_status(s.id, GameServerStatus::Error, Some(reason))
                .await;
            let _ = ctx.session_repo.close_all_active(s.id).await;
            mark_crashed(ctx, s).await;
            return true;
        }

        // Sous le plafond mais cooldown de backoff non ecoule : on ne fait
        // RIEN ce tick (le serveur reste Running et sera re-evalue au prochain
        // passage, une fois le backoff ecoule). Evite de marteler le restart.
        if !should_auto_restart(
            auto_restart_on_crash,
            s.restart_attempts,
            max_attempts,
            s.last_restart_at,
            now,
            backoff_base_secs,
            backoff_cap_secs,
        ) {
            info!(
                server_id = %s.id,
                attempts = s.restart_attempts,
                "crash detecte, backoff en cours, attente avant redemarrage"
            );
            return false;
        }

        // On peut redemarrer. Il faut un container_id a relancer ; sans lui on
        // ne peut pas auto-restart -> abandon (Error).
        let Some(cid) = s.container_id.clone() else {
            warn!(server_id = %s.id, "crash sans container_id, impossible de redemarrer");
            let _ = ctx
                .server_repo
                .update_status(
                    s.id,
                    GameServerStatus::Error,
                    Some("crash : pas de container_id pour redemarrer (reconciler)"),
                )
                .await;
            let _ = ctx.session_repo.close_all_active(s.id).await;
            mark_crashed(ctx, s).await;
            return true;
        };

        // Comptabilise la tentative AVANT le start (pose last_restart_at -> le
        // backoff s'applique meme si le start echoue : pas de boucle serree).
        if let Err(e) = ctx.server_repo.record_restart_attempt(s.id).await {
            warn!(error = %e, server_id = %s.id, "record_restart_attempt");
        }
        let attempt = s.restart_attempts + 1;
        info!(
            server_id = %s.id,
            attempt,
            max = max_attempts,
            "crash detecte, auto-restart du container"
        );
        let _ = ctx.session_repo.close_all_active(s.id).await;

        match ctx.container_runtime.start_container(&cid).await {
            Ok(()) => {
                let _ = ctx
                    .server_repo
                    .update_runtime(
                        s.id,
                        GameServerRuntimeUpdate {
                            status: Some(GameServerStatus::Starting),
                            clear_last_error: true,
                            started_at_now: true,
                            ..Default::default()
                        },
                    )
                    .await;
                let _ = ctx
                    .audit_repo
                    .log(
                        &s.guild_id,
                        Some(s.id),
                        None,
                        GameAuditAction::AutoRestart,
                        serde_json::json!({ "attempt": attempt, "max": max_attempts }),
                    )
                    .await;
                false
            }
            Err(e) => {
                // Echec du start : on laisse le serveur en Running ; le backoff
                // (last_restart_at vient d'etre pose) gate la prochaine
                // tentative au prochain tick, toujours borne par le plafond.
                warn!(error = %e, server_id = %s.id, attempt, "auto-restart start_container echoue");
                true
            }
        }
    }

    // 1. DB -> Docker : reconcilie chaque serveur actif avec son container.
    for s in &active_servers {
        // Config per-guild : seuil "stuck" + parametres d'auto-restart. Chargee
        // une fois par serveur et reutilisee ci-dessous (stuck + crash).
        let cfg = load_game_portal_config(&ctx.bot_config, &s.guild_id).await?;
        let dc = docker_by_id.get(&s.id.to_string());
        // Serveur coince dans un etat transitoire depuis trop longtemps ?
        let stuck = (now - s.updated_at)
            > chrono::Duration::minutes(cfg.stuck_transition_threshold_minutes);
        match dc {
            None => match s.status {
                // Container disparu alors qu'on le croyait up -> crash.
                GameServerStatus::Running => {
                    warn!(server_id = %s.id, "container disparu, marque error");
                    let _ = ctx
                        .server_repo
                        .update_status(
                            s.id,
                            GameServerStatus::Error,
                            Some("container disparu (reconciler)"),
                        )
                        .await;
                    mark_crashed(ctx, s).await;
                    errors += 1;
                }
                // Starting sans container : NORMAL en plein milieu d'un start
                // (flow persist-after-create). On ne tranche qu'au-dela du
                // seuil pour ne pas tuer un demarrage en cours.
                GameServerStatus::Starting if stuck => {
                    warn!(server_id = %s.id, "starting bloque sans container, marque error");
                    let _ = ctx
                        .server_repo
                        .update_status(
                            s.id,
                            GameServerStatus::Error,
                            Some("starting bloque (reconciler)"),
                        )
                        .await;
                    mark_crashed(ctx, s).await;
                    errors += 1;
                }
                // Stopping sans container : l'arret a abouti, on finalise.
                GameServerStatus::Stopping => {
                    let _ = ctx
                        .server_repo
                        .update_status(
                            s.id,
                            GameServerStatus::Stopped,
                            Some("container absent au stop (reconciler)"),
                        )
                        .await;
                    let _ = ctx.session_repo.close_all_active(s.id).await;
                }
                _ => {}
            },
            Some(c) => {
                let exited = matches!(c.state, ContainerState::Exited | ContainerState::Dead);
                let running = c.state == ContainerState::Running;
                match s.status {
                    // Running mais container mort -> crash. L'etat DESIRE est
                    // Running (l'utilisateur ne l'a pas stoppe) : on tente un
                    // auto-restart borne + backoff plutot que de juste stopper.
                    GameServerStatus::Running
                        if exited
                            && handle_running_crash(
                                ctx,
                                s,
                                now,
                                cfg.max_auto_restart_attempts,
                                cfg.auto_restart_on_crash,
                                cfg.restart_backoff_base_secs,
                                cfg.restart_backoff_cap_secs,
                            )
                            .await =>
                    {
                        errors += 1;
                    }
                    // Running et container running : observation saine. Si des
                    // tentatives de redemarrage etaient en cours, on les remet
                    // a 0 (cheap : seulement si restart_attempts > 0). Couvre
                    // les serveurs sans RCON, non vus par run_health_check.
                    GameServerStatus::Running if running && s.restart_attempts > 0 => {
                        if let Err(e) = ctx.server_repo.reset_restart_attempts(s.id).await {
                            warn!(error = %e, server_id = %s.id, "reset_restart_attempts");
                        }
                    }
                    // Starting bloque : on resout selon l'etat reel.
                    GameServerStatus::Starting if stuck && running => {
                        let _ = ctx
                            .server_repo
                            .update_status(s.id, GameServerStatus::Running, None)
                            .await;
                    }
                    GameServerStatus::Starting if stuck && exited => {
                        let _ = ctx
                            .server_repo
                            .update_status(
                                s.id,
                                GameServerStatus::Error,
                                Some("starting bloque, container exited (reconciler)"),
                            )
                            .await;
                        errors += 1;
                    }
                    // Stopping : Stopped immédiatement dès que le container est mort (exited),
                    // pas besoin d'attendre le seuil de 10 minutes (stuck).
                    GameServerStatus::Stopping if exited => {
                        let _ = ctx
                            .server_repo
                            .update_status(
                                s.id,
                                GameServerStatus::Stopped,
                                Some("container absent/exited suite a l'arret (reconciler)"),
                            )
                            .await;
                        let _ = ctx.session_repo.close_all_active(s.id).await;
                    }
                    GameServerStatus::Stopping if stuck && running => {
                        let _ = ctx
                            .server_repo
                            .update_status(s.id, GameServerStatus::Running, None)
                            .await;
                    }
                    _ => {}
                }
            }
        }
    }

    // 2. Docker -> DB : containers managed sans ligne game_servers vivante
    // (orphelins) -> on les SUPPRIME best-effort. Un container dont le
    // server_id ne resout vers aucune ligne non-deletee (find_by_id filtre
    // deleted_at IS NULL) est soit un reliquat d'un delete dont le remove a
    // echoue, soit un container etranger : dans les deux cas on le retire.
    // On NE TOUCHE PAS aux containers qui mappent vers un serveur vivant
    // (meme stopped : sa ligne existe encore et find_by_id la retourne).
    let mut orphans = 0usize;
    for c in &docker_containers {
        let Some(sid) = c.labels.get("sentinel.server_id") else {
            continue;
        };
        let is_live = match Uuid::parse_str(sid) {
            Ok(uid) => ctx.server_repo.find_by_id(uid).await?.is_some(),
            // Label illisible -> on considere comme orphelin.
            Err(_) => false,
        };
        if is_live {
            continue;
        }
        warn!(container_id = %c.container_id, server_id = %sid, "orphelin Docker, suppression");
        if let Err(e) = ctx
            .container_runtime
            .remove_container(&c.container_id)
            .await
        {
            warn!(error = %e, container_id = %c.container_id, "remove orphelin echoue");
            errors += 1;
        }
        orphans += 1;
    }
    details.insert("orphans".into(), serde_json::json!(orphans));
    details.insert("active_db".into(), serde_json::json!(active_servers.len()));
    details.insert(
        "managed_docker".into(),
        serde_json::json!(docker_containers.len()),
    );

    Ok(JobReport {
        job: "reconciler",
        processed: active_servers.len(),
        errors,
        details: serde_json::Value::Object(details),
    })
}

// ════════════════════════════════════════════════════════════════════════
// JOB 4 : IMAGE CLEANUP
// ════════════════════════════════════════════════════════════════════════

/// Pour chaque template du catalogue, regarde s'il existe encore des
/// serveurs actifs qui utilisent ce template. Si non, et si la derniere
/// activite est plus ancienne que `unused_image_grace_days`, supprime
/// l'image Docker. Docker refusera la suppression si un container l'utilise
/// encore (defense en profondeur).
pub async fn run_image_cleanup(ctx: &JobContext) -> Result<JobReport, DomainError> {
    // Lecture de la config global (defaut sentinel-* sans guild — on prend
    // la premiere guild qui a une config game-portal). Pour rester simple,
    // on prend les defaults via une guild fictive : ils s'appliquent sauf
    // si l'admin a override.
    let cfg = load_game_portal_config(&ctx.bot_config, "_global").await?;
    if !cfg.auto_remove_unused_images {
        return Ok(JobReport {
            job: "image_cleanup",
            processed: 0,
            errors: 0,
            details: serde_json::json!({"skipped": "auto_remove_unused_images=false"}),
        });
    }
    let grace_days = cfg.unused_image_grace_days;
    if grace_days <= 0 {
        return Ok(JobReport {
            job: "image_cleanup",
            processed: 0,
            errors: 0,
            details: serde_json::json!({"skipped": "grace_days <= 0"}),
        });
    }

    let templates = ctx.template_repo.list().await?;
    let now = chrono::Utc::now();
    let mut removed = 0usize;
    let mut errors = 0usize;
    let mut details = serde_json::Map::new();

    for tpl in &templates {
        let usage = ctx.server_repo.template_usage(tpl.id).await?;
        if usage.active_count > 0 {
            continue;
        }
        let last = match usage.last_activity_at {
            Some(t) => t,
            None => continue, // template jamais utilise, image jamais pull -> rien a faire
        };
        let cutoff = now - chrono::Duration::days(grace_days as i64);
        if last >= cutoff {
            // Activite trop recente, on respecte la grace period.
            continue;
        }

        info!(template = %tpl.slug, image = %tpl.image, days = grace_days, "image cleanup");
        match ctx.container_runtime.remove_image(&tpl.image, false).await {
            Ok(true) => {
                removed += 1;
                details.insert(tpl.slug.clone(), serde_json::json!("removed"));
                let _ = ctx
                    .audit_repo
                    .log(
                        "_global",
                        None,
                        None,
                        crate::domain::entities::game::audit::GameAuditAction::Delete,
                        serde_json::json!({
                            "kind": "image_cleanup",
                            "template": tpl.slug,
                            "image": tpl.image,
                        }),
                    )
                    .await;
            }
            Ok(false) => {
                details.insert(tpl.slug.clone(), serde_json::json!("not_present"));
            }
            Err(e) => {
                warn!(error = %e, template = %tpl.slug, "image_cleanup failed");
                errors += 1;
                details.insert(tpl.slug.clone(), serde_json::json!(format!("error: {e}")));
            }
        }
    }

    Ok(JobReport {
        job: "image_cleanup",
        processed: removed,
        errors,
        details: serde_json::Value::Object(details),
    })
}

// ════════════════════════════════════════════════════════════════════════
// JOB 5 : REVEAL IP
// ════════════════════════════════════════════════════════════════════════

pub async fn run_reveal_ip(ctx: &JobContext) -> Result<JobReport, DomainError> {
    use crate::ports::outbound::events::game_events::IP_REVEAL;

    let due = ctx.server_repo.list_ip_reveal_due().await?;
    let mut processed = 0usize;
    let mut errors = 0usize;
    let mut servers = Vec::new();

    for s in &due {
        if let Err(e) = ctx.server_repo.mark_ip_revealed(s.id).await {
            warn!(error = %e, server_id = %s.id, "reveal_ip: mark echoue, skip");
            errors += 1;
            continue;
        }
        let payload = serde_json::json!({
            "server_id": s.id.to_string(),
            "guild_id": s.guild_id,
        });
        ctx.events.publish(IP_REVEAL, payload.clone()).await;
        servers.push(payload);
        processed += 1;
    }

    Ok(JobReport {
        job: "reveal_ip",
        processed,
        errors,
        details: serde_json::json!({ "servers": servers }),
    })
}

// ════════════════════════════════════════════════════════════════════════
// JOB 6 : DAILY PING
// ════════════════════════════════════════════════════════════════════════

pub async fn run_daily_ping(ctx: &JobContext) -> Result<JobReport, DomainError> {
    use crate::domain::entities::system::bot_config::{cfg_bool, cfg_i64};
    use crate::ports::outbound::events::game_events::DAILY_PING;
    use chrono::Timelike;

    let now_hour = chrono::Utc::now().hour() as i64;
    let awaiting = ctx.server_repo.list_awaiting_reveal_no_ping_today().await?;
    let mut processed = 0usize;
    let mut errors = 0usize;
    let mut servers = Vec::new();

    for s in &awaiting {
        let cfg = ctx
            .bot_config
            .get_config(&s.guild_id, "game-portal")
            .await
            .unwrap_or_default();
        let enabled = cfg_bool(&cfg, "session_daily_ping_enabled", false);
        let hour = cfg_i64(&cfg, "session_daily_ping_hour", 18);

        if enabled && now_hour >= hour {
            if let Err(e) = ctx.server_repo.mark_daily_ping(s.id).await {
                warn!(error = %e, server_id = %s.id, "daily_ping: mark echoue, skip");
                errors += 1;
                continue;
            }
            let payload = serde_json::json!({
                "server_id": s.id.to_string(),
                "guild_id": s.guild_id,
            });
            ctx.events.publish(DAILY_PING, payload.clone()).await;
            servers.push(payload);
            processed += 1;
        }
    }

    Ok(JobReport {
        job: "daily_ping",
        processed,
        errors,
        details: serde_json::json!({ "servers": servers }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::game::audit::{GameAuditAction, GameAuditEntry};
    use crate::domain::entities::game::player_session::PlayerSession;
    use crate::domain::entities::game::server::{GameServer, GameServerStatus};
    use crate::domain::entities::system::bot_config::{BotDefinition, BotGuildConfig};
    use crate::ports::outbound::game::container_runtime::{
        ContainerSpec, ContainerStats, ContainerStatus, ManagedContainer,
    };
    use crate::ports::outbound::game::game_server_repository::{
        GameServerRuntimeUpdate, NewGameServer, TemplateUsage,
    };

    #[test]
    fn test_parse_minecraft_list() {
        let raw = "There are 2 of a max of 20 players online: Alice, Bob";
        let (count, players) = parse_minecraft_list(raw);
        assert_eq!(count, 2);
        assert_eq!(players, vec!["Alice", "Bob"]);

        let raw_empty = "There are 0 of a max of 20 players online:";
        let (count2, players2) = parse_minecraft_list(raw_empty);
        assert_eq!(count2, 0);
        assert!(players2.is_empty());
    }

    struct DummyServerRepo;
    #[async_trait::async_trait]
    impl GameServerRepository for DummyServerRepo {
        async fn create(&self, _: NewGameServer) -> Result<GameServer, DomainError> {
            todo!()
        }
        async fn find_by_id(&self, _: Uuid) -> Result<Option<GameServer>, DomainError> {
            Ok(None)
        }
        async fn list_by_guild(&self, _: &str) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![])
        }
        async fn list_running(&self) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![])
        }
        async fn list_active(&self) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![])
        }
        async fn update_runtime(
            &self,
            _: Uuid,
            _: GameServerRuntimeUpdate,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn update_status(
            &self,
            _: Uuid,
            _: GameServerStatus,
            _: Option<&str>,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn try_transition_status(
            &self,
            _: Uuid,
            _: &[GameServerStatus],
            _: GameServerStatus,
        ) -> Result<bool, DomainError> {
            Ok(true)
        }
        async fn update_player_activity(&self, _: Uuid, _: i32) -> Result<(), DomainError> {
            Ok(())
        }
        async fn record_restart_attempt(&self, _: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn reset_restart_attempts(&self, _: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn soft_delete(&self, _: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn count_active_for_guild(&self, _: &str) -> Result<(i32, i32), DomainError> {
            Ok((0, 0))
        }
        async fn template_usage(&self, _: Uuid) -> Result<TemplateUsage, DomainError> {
            Ok(TemplateUsage {
                active_count: 0,
                last_activity_at: None,
            })
        }
        async fn set_session_channels(
            &self,
            _: Uuid,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<bool, DomainError> {
            Ok(true)
        }
        async fn mark_ip_revealed(&self, _: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn list_ip_reveal_due(&self) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![])
        }
        async fn list_awaiting_reveal_no_ping_today(&self) -> Result<Vec<GameServer>, DomainError> {
            Ok(vec![])
        }
        async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn set_ip_reveal_at(
            &self,
            _: Uuid,
            _: Option<chrono::DateTime<chrono::Utc>>,
        ) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct DummyEventPublisher;
    #[async_trait::async_trait]
    impl crate::ports::outbound::events::EventPublisher for DummyEventPublisher {
        async fn publish(&self, _: &str, _: serde_json::Value) {}
    }

    struct DummyBotConfig;
    #[async_trait::async_trait]
    impl BotConfigRepository for DummyBotConfig {
        async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
            Ok(vec![])
        }
        async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
            Ok(vec![])
        }
        async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
            Ok(vec![])
        }
        async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct DummyTemplateRepo;
    #[async_trait::async_trait]
    impl crate::ports::outbound::game::game_template_repository::GameTemplateRepository
        for DummyTemplateRepo
    {
        async fn list(
            &self,
        ) -> Result<Vec<crate::domain::entities::game::template::GameTemplate>, DomainError>
        {
            Ok(vec![])
        }
        async fn find_by_id(
            &self,
            _: Uuid,
        ) -> Result<Option<crate::domain::entities::game::template::GameTemplate>, DomainError>
        {
            Ok(None)
        }
        async fn find_by_slug(
            &self,
            _: &str,
        ) -> Result<Option<crate::domain::entities::game::template::GameTemplate>, DomainError>
        {
            Ok(None)
        }
    }

    struct DummyAuditRepo;
    #[async_trait::async_trait]
    impl GameAuditRepository for DummyAuditRepo {
        async fn log(
            &self,
            _: &str,
            _: Option<Uuid>,
            _: Option<&str>,
            _: GameAuditAction,
            _: serde_json::Value,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn list_for_server(
            &self,
            _: Uuid,
            _: i64,
        ) -> Result<Vec<GameAuditEntry>, DomainError> {
            Ok(vec![])
        }
        async fn list_for_guild(
            &self,
            _: &str,
            _: i64,
            _: i64,
        ) -> Result<Vec<GameAuditEntry>, DomainError> {
            Ok(vec![])
        }
    }

    struct DummySessionRepo;
    #[async_trait::async_trait]
    impl PlayerSessionRepository for DummySessionRepo {
        async fn open(&self, _: Uuid, _: &str) -> Result<Uuid, DomainError> {
            Ok(Uuid::new_v4())
        }
        async fn close(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn list_active(&self, _: Uuid) -> Result<Vec<PlayerSession>, DomainError> {
            Ok(vec![])
        }
        async fn list_history(
            &self,
            _: Uuid,
            _: i64,
            _: i64,
        ) -> Result<Vec<PlayerSession>, DomainError> {
            Ok(vec![])
        }
        async fn close_all_active(&self, _: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct DummyRuntime;
    #[async_trait::async_trait]
    impl ContainerRuntime for DummyRuntime {
        async fn ensure_network(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn ensure_volume(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn pull_image_if_missing(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn create_container(&self, _: &ContainerSpec) -> Result<String, DomainError> {
            Ok("id".into())
        }
        async fn start_container(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn upload_file_to_container(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn stop_container(&self, _: &str, _: u32) -> Result<(), DomainError> {
            Ok(())
        }
        async fn restart_container(&self, _: &str, _: u32) -> Result<(), DomainError> {
            Ok(())
        }
        async fn remove_container(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn remove_volume(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn remove_image(&self, _: &str, _: bool) -> Result<bool, DomainError> {
            Ok(true)
        }
        async fn inspect(&self, _: &str) -> Result<Option<ContainerStatus>, DomainError> {
            Ok(None)
        }
        async fn stats(&self, _: &str) -> Result<ContainerStats, DomainError> {
            todo!()
        }
        async fn logs(&self, _: &str, _: u32) -> Result<Vec<String>, DomainError> {
            Ok(vec![])
        }
        async fn list_managed_containers(&self) -> Result<Vec<ManagedContainer>, DomainError> {
            Ok(vec![])
        }
    }

    struct DummyRcon;
    #[async_trait::async_trait]
    impl RconClient for DummyRcon {
        async fn execute(
            &self,
            _: &RconConnectionParams,
            _: &str,
        ) -> Result<crate::ports::outbound::game::rcon_client::RconResponse, DomainError> {
            todo!()
        }
    }

    struct DummyPortAllocator;
    #[async_trait::async_trait]
    impl PortAllocator for DummyPortAllocator {
        async fn allocate(&self, _: PortKind, _: u16, _: u16, _: &str) -> Result<u16, DomainError> {
            Ok(25565)
        }
        async fn release(&self, _: PortKind, _: u16) -> Result<(), DomainError> {
            Ok(())
        }
        async fn is_available(&self, _: PortKind, _: u16) -> Result<bool, DomainError> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_run_reveal_ip_empty() {
        let ctx = JobContext {
            server_repo: Arc::new(DummyServerRepo),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(DummyRuntime),
            rcon_client: Arc::new(DummyRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(DummyBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_reveal_ip(&ctx).await.unwrap();
        assert_eq!(report.job, "reveal_ip");
        assert_eq!(report.processed, 0);
        assert_eq!(report.errors, 0);
    }

    #[tokio::test]
    async fn test_run_daily_ping_empty() {
        let ctx = JobContext {
            server_repo: Arc::new(DummyServerRepo),
            template_repo: Arc::new(DummyTemplateRepo),
            audit_repo: Arc::new(DummyAuditRepo),
            session_repo: Arc::new(DummySessionRepo),
            container_runtime: Arc::new(DummyRuntime),
            rcon_client: Arc::new(DummyRcon),
            port_allocator: Arc::new(DummyPortAllocator),
            bot_config: Arc::new(DummyBotConfig),
            events: Arc::new(DummyEventPublisher),
        };
        let report = run_daily_ping(&ctx).await.unwrap();
        assert_eq!(report.job, "daily_ping");
        assert_eq!(report.processed, 0);
        assert_eq!(report.errors, 0);
    }
}
