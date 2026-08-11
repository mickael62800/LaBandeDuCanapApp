//! Job publication du classement mensuel (texte / vocal / global).
//!
//! Tick periodique : l'API decide quoi faire (gate `monthly_ranking_enabled`,
//! passage de mois, salon configure, pose de baseline). Le worker ne fait que
//! reveiller l'API.

use sqlx::PgPool;
use tracing::info;

use platform_common_worker::api;

#[derive(serde::Deserialize)]
struct JobReport {
    guilds_published: usize,
    guilds_baselined: usize,
    guilds_skipped: usize,
}

pub async fn run(_pool: &PgPool) -> Result<(), String> {
    let report: JobReport = api::post_empty("/api/analytics/publish-monthly-ranking").await?;
    if report.guilds_published > 0 || report.guilds_baselined > 0 {
        info!(
            published = report.guilds_published,
            baselined = report.guilds_baselined,
            skipped = report.guilds_skipped,
            "Classement mensuel : publication / baseline effectuees"
        );
    }
    Ok(())
}
