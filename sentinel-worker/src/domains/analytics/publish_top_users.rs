//! Job publication Top users : tick periodique, l'API decide quoi
//! publier (gates `top_users_publish_enabled`, intervalle ecoule, salon
//! configure). Le worker ne fait que reveiller l'API.

use sqlx::PgPool;
use tracing::info;

use super::GuildJobReport;
use platform_common_worker::api;

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    let report: GuildJobReport = api::post_empty("/api/analytics/publish-top-users").await?;
    if report.guilds_processed > 0 {
        info!(
            published = report.guilds_processed,
            skipped = report.guilds_skipped,
            "Publication Top users effectuee"
        );
    }
    Ok(())
}
