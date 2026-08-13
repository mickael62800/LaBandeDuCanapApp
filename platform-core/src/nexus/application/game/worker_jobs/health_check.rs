use super::*;

// ════════════════════════════════════════════════════════════════════════
// JOB 1 : HEALTH CHECK
// ════════════════════════════════════════════════════════════════════════

/// Pour chaque serveur `running`, query player count via RCON `list`. Met
/// a jour last_player_count + last_active_at, ouvre/ferme les sessions.
pub async fn run_health_check(ctx: &JobContext) -> Result<JobReport, DomainError> {
    let servers = ctx.server_repo.list_running().await?;
    let configs = load_game_portal_configs(
        &ctx.bot_config,
        servers.iter().map(|server| server.guild_id.as_str()),
    )
    .await?;
    let mut errors = 0usize;
    let mut details = serde_json::Map::new();

    for server in &servers {
        let cfg = &configs[&server.guild_id];
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
pub(super) fn parse_minecraft_list(raw: &str) -> (i32, Vec<String>) {
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
