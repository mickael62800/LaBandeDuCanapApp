//! Job de retention analytics : delegue a l'API qui purge daily_activity
//! / hourly_activity selon `data_retention_days` par guild.

use sqlx::PgPool;
use tracing::info;

use super::GuildJobReport;
use platform_common_worker::api;

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    let report: GuildJobReport = api::post_empty("/api/analytics/retention-cleanup").await?;
    info!(
        processed = report.guilds_processed,
        skipped = report.guilds_skipped,
        "Retention analytics deleguee API"
    );
    Ok(())
}
