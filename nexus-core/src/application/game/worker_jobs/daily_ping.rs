use super::*;

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

    // Le ping quotidien reste tolerant aux erreurs de lecture comme avant,
    // mais ne refait plus la meme requete pour chaque session d'une guild.
    let guild_ids = awaiting
        .iter()
        .map(|server| server.guild_id.as_str())
        .collect::<HashSet<_>>();
    let mut configs = HashMap::with_capacity(guild_ids.len());
    for guild_id in guild_ids {
        let config = ctx
            .bot_config
            .get_config(guild_id, "game-portal")
            .await
            .unwrap_or_default();
        configs.insert(guild_id.to_string(), config);
    }

    for s in &awaiting {
        let cfg = &configs[&s.guild_id];
        let enabled = cfg_bool(cfg, "session_daily_ping_enabled", false);
        let hour = cfg_i64(cfg, "session_daily_ping_hour", 18);

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
