//! Job de retention des `announcement_runs` : delegue a l'API qui purge
//! par guild selon `history_retention_days` (cle `announcements`).

use sqlx::PgPool;
use tracing::info;

use platform_common_worker::api;

#[derive(serde::Deserialize)]
struct JobReport {
    guilds_processed: u64,
    guilds_skipped: u64,
    rows_deleted: i64,
}

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    let report: JobReport =
        api::post_empty("/api/announcements/internal/retention-cleanup").await?;
    info!(
        processed = report.guilds_processed,
        skipped = report.guilds_skipped,
        deleted = report.rows_deleted,
        "Retention announcement_runs deleguee API"
    );
    Ok(())
}
