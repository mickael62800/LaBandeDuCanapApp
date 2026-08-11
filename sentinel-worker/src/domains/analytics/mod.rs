//! Domaine analytics : snapshots quotidien et horaire (calculs
//! d'agregats long terme : daily_activity, hourly_activity), retention
//! et publication automatique du Top users sur Discord.
//!
//! Tout le metier est dans l'API — les jobs ici sont des tickers qui
//! POST vers les endpoints `/api/analytics/*`.

#[derive(serde::Deserialize)]
pub(super) struct GuildJobReport {
    pub guilds_processed: usize,
    pub guilds_skipped: usize,
}

pub mod daily_snapshot;
pub mod hourly_snapshot;
pub mod publish_monthly_ranking;
pub mod publish_top_users;
pub mod retention_cleanup;
