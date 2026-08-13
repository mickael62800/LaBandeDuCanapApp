use serde::Deserialize;

use crate::config::DomainConfig;

#[derive(Deserialize)]
struct SummaryResponse {
    summary: String,
    generated_by_ai: bool,
}

#[derive(Deserialize)]
struct RetentionResponse {
    ok: bool,
}

pub fn start(config: DomainConfig) {
    let guild_id = std::env::var("ATRIUM_PRIMARY_GUILD_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("PUBLIC_GUILD_ID").ok())
        .unwrap_or_default();
    if guild_id.is_empty() {
        tracing::error!("Atrium active sans ATRIUM_PRIMARY_GUILD_ID/PUBLIC_GUILD_ID");
        return;
    }

    let summary_client = config.client.clone();
    crate::schedule::spawn_interval(
        "atrium.server-summary",
        crate::schedule::env_u64("ATRIUM_SUMMARY_INTERVAL_SECS", 86_400),
        move || {
            let client = summary_client.clone();
            let path = format!("/admin/guilds/{guild_id}/jobs/summary");
            async move {
                let response: SummaryResponse = client.post_json(&path).await?;
                tracing::info!(
                    ai = response.generated_by_ai,
                    size = response.summary.len(),
                    "resume Atrium genere"
                );
                Ok(())
            }
        },
    );

    let retention_client = config.client;
    crate::schedule::spawn_interval(
        "atrium.retention",
        crate::schedule::env_u64("ATRIUM_RETENTION_INTERVAL_SECS", 86_400),
        move || {
            let client = retention_client.clone();
            async move {
                let response: RetentionResponse = client.post_json("/admin/jobs/retention").await?;
                tracing::info!(ok = response.ok, "retention Atrium executee");
                Ok(())
            }
        },
    );
}
