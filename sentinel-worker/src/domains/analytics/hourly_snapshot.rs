//! Snapshot horaire : delegation a l'API (cf. daily_snapshot).

use sqlx::PgPool;
use tracing::info;

use super::GuildJobReport;
use platform_common_worker::api;

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    let report: GuildJobReport = api::post_empty("/api/analytics/snapshot/hourly").await?;
    if report.guilds_processed > 0 || report.guilds_skipped > 0 {
        info!(
            processed = report.guilds_processed,
            skipped = report.guilds_skipped,
            "Snapshot horaire delegue API"
        );
    }
    Ok(())
}
