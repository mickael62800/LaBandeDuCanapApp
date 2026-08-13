use super::*;

// ════════════════════════════════════════════════════════════════════════
// JOB 2 : IDLE SHUTDOWN
// ════════════════════════════════════════════════════════════════════════

/// Stop les serveurs running dont `last_active_at` est anterieur a
/// `idle_shutdown_days` jours (override par instance ou template).
pub async fn run_idle_shutdown(ctx: &JobContext) -> Result<JobReport, DomainError> {
    let servers = ctx.server_repo.list_running().await?;
    let configs = load_game_portal_configs(
        &ctx.bot_config,
        servers.iter().map(|server| server.guild_id.as_str()),
    )
    .await?;
    let mut stopped = 0usize;
    let mut errors = 0usize;

    let now = chrono::Utc::now();
    for server in &servers {
        // Resoud le seuil idle (instance override -> template default).
        let cfg = &configs[&server.guild_id];
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
