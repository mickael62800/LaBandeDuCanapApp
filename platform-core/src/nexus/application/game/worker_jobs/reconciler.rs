use super::*;

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
    let configs = load_game_portal_configs(
        &ctx.bot_config,
        active_servers.iter().map(|server| server.guild_id.as_str()),
    )
    .await?;
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
        .filter_map(|c| managed_server_id_label(&c.labels).map(|sid| (sid.to_string(), c)))
        .collect();

    // Helper local : libere les ports d'un serveur + log un crash audit.
    async fn mark_crashed(
        ctx: &JobContext,
        s: &crate::nexus::domain::entities::game::server::GameServer,
    ) {
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
        s: &crate::nexus::domain::entities::game::server::GameServer,
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
        // Config per-guild reutilisee pour tous les serveurs de la guild.
        let cfg = &configs[&s.guild_id];
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
    // server_id ne resout vers aucune ligne non-deletee est soit un reliquat
    // d'un delete dont le remove a
    // echoue, soit un container etranger : dans les deux cas on le retire.
    // On NE TOUCHE PAS aux containers qui mappent vers un serveur vivant
    // (meme stopped : sa ligne existe encore). La verification est faite en
    // une seule requete pour tous les identifiants Docker valides.
    let referenced_server_ids = docker_containers
        .iter()
        .filter_map(|container| managed_server_id_label(&container.labels))
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let existing_server_ids = ctx
        .server_repo
        .find_existing_ids(&referenced_server_ids)
        .await?;

    let mut orphans = 0usize;
    for c in &docker_containers {
        let Some(sid) = managed_server_id_label(&c.labels) else {
            continue;
        };
        let is_live = Uuid::parse_str(sid)
            .ok()
            .is_some_and(|id| existing_server_ids.contains(&id));
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
