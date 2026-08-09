//! Job publication Top users : tick periodique, l'API decide quoi
//! publier (gates `top_users_publish_enabled`, intervalle ecoule, salon
//! configure). Le worker ne fait que reveiller l'API.

use sqlx::PgPool;
use tracing::info;

use platform_common_worker::api;

#[derive(serde::Deserialize)]
struct JobReport {
    guilds_processed: usize,
    guilds_skipped: usize,
}

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    let report: JobReport = api::post_empty("/api/analytics/publish-top-users").await?;
    if report.guilds_processed > 0 {
        info!(
            published = report.guilds_processed,
            skipped = report.guilds_skipped,
            "Publication Top users effectuee"
        );
    }
    Ok(())
}

