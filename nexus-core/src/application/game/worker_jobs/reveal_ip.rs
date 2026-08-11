use super::*;

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
