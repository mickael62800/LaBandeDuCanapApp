use super::*;
use crate::nexus::domain::entities::game::presence;

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
        // La commande ET le format de reponse dependent du jeu : Palworld
        // repond a `ShowPlayers`, pas a `list`. Interroger avec la mauvaise
        // commande renvoyait « 0 joueur » sur un serveur peuple, ce qui
        // alimente l'extinction automatique — donc eteint un serveur occupe.
        let slug = ctx
            .template_repo
            .find_by_id(server.template_id)
            .await
            .ok()
            .flatten()
            .map(|t| t.slug)
            .unwrap_or_default();
        let commande = presence::players_command(&slug);

        let resp = match ctx.rcon_client.execute(&params, commande).await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, server_id = %server.id, "health rcon failed");
                errors += 1;
                continue;
            }
        };
        let presents = presence::parse_players(&slug, &resp.raw);
        let count = presents.len() as i32;
        let players: Vec<String> = presents.into_iter().map(|p| p.name).collect();

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
