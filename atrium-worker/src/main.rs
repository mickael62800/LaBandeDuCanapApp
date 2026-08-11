//! Worker de fond de la plateforme d'accueil Atrium.

use std::time::Duration;

use platform_common_worker::http_job::HttpJobClient;
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
struct SummaryJobResponse {
    summary: String,
    generated_by_ai: bool,
}

#[derive(Debug, Deserialize)]
struct RetentionJobResponse {
    ok: bool,
}

#[tokio::main]
async fn main() {
    platform_common_worker::init_tracing("atrium_worker=info");
    platform_common_worker::metrics::init_observability("atrium-worker");

    let api_url =
        std::env::var("ATRIUM_API_URL").unwrap_or_else(|_| "http://localhost:8090".into());
    let api_token = std::env::var("ATRIUM_API_TOKEN").unwrap_or_default();
    let guild_id = std::env::var("ATRIUM_PRIMARY_GUILD_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("PUBLIC_GUILD_ID")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| {
            tracing::error!(
                "ATRIUM_PRIMARY_GUILD_ID ou PUBLIC_GUILD_ID doit identifier la guilde a resumer"
            );
            std::process::exit(2);
        });
    let interval_secs = platform_common_worker::env_u64("ATRIUM_SUMMARY_INTERVAL_SECS", 86_400);
    let retention_secs =
        platform_common_worker::env_u64("ATRIUM_RETENTION_INTERVAL_SECS", 86_400);
    let client = HttpJobClient::new(api_url.clone(), api_token, Duration::from_secs(30));

    info!(%api_url, %guild_id, interval_secs, "atrium-worker demarre");
    let summary_client = client.clone();
    platform_common_worker::spawn_interval("server-summary", interval_secs, move || {
        let client = summary_client.clone();
        let path = format!("/admin/guilds/{guild_id}/jobs/summary");
        async move {
            let response: SummaryJobResponse = client.post_json(&path).await?;
            info!(
                ai = response.generated_by_ai,
                taille = response.summary.len(),
                "Resume meteo genere via Atrium API"
            );
            Ok(())
        }
    });

    // Purge quotidienne des compteurs de quota, sortie du chemin critique du
    // budget (cf. `BudgetGuard::purge_old`). Sans guilde : maintenance globale.
    platform_common_worker::spawn_interval("budget-retention", retention_secs, move || {
        let client = client.clone();
        async move {
            let response: RetentionJobResponse = client.post_json("/admin/jobs/retention").await?;
            info!(ok = response.ok, "Purge des quotas Atrium effectuee");
            Ok(())
        }
    });

    platform_common_worker::shutdown_signal().await;
    info!("atrium-worker arrete");
}
