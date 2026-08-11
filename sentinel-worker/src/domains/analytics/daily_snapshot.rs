//! Snapshot quotidien : delegation a l'API.
//!
//! Le metier (gating track_voice_stats / track_message_stats par guild,
//! INSERT daily_activity) vit dans `sentinel-api/src/adapters/inbound/
//! http/handlers/audit/snapshots.rs::snapshot_daily_all`. Ce job se
//! contente de tick et POST.

use sqlx::PgPool;
use tracing::info;

use super::GuildJobReport;
use platform_common_worker::api;

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    let report: GuildJobReport = api::post_empty("/api/analytics/snapshot/daily").await?;
    if report.guilds_processed > 0 || report.guilds_skipped > 0 {
        info!(
            processed = report.guilds_processed,
            skipped = report.guilds_skipped,
            "Snapshot quotidien delegue API"
        );
    }
    Ok(())
}
