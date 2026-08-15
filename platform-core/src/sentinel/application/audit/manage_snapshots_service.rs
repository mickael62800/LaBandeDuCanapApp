//! Use case des jobs analytics : snapshots quotidien/horaire, purge de
//! retention et calcul des publications "Top users". La lecture de config par
//! guild et toute la regle metier vivent ici ; le SQL vit dans
//! `SnapshotRepository`, le POST Discord reste au handler.

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::snapshot::{
    JobReport, TopPublication, TopPublishPlan,
};
use crate::sentinel::domain::entities::system::bot_config::BotGuildConfig;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::audit::manage_snapshots::ManageSnapshotsUseCase;
use crate::sentinel::ports::outbound::audit::analytics_repository::AnalyticsRepository;
use crate::sentinel::ports::outbound::audit::snapshot_repository::SnapshotRepository;
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;

use crate::sentinel::domain::entities::system::bot_names::ANALYTICS_BOT;
/// Cle de state (hors schema UI) memorisant le dernier post Top users.
const LAST_PUBLISH_KEY: &str = "top_users_last_published_at";
/// Couleur de l'embed Top infracteurs (rouge Discord).
const TOP_EMBED_COLOR: u32 = 0xED4245;

pub struct ManageSnapshotsService {
    config: Arc<dyn BotConfigRepository>,
    repo: Arc<dyn SnapshotRepository>,
    analytics: Arc<dyn AnalyticsRepository>,
}

impl ManageSnapshotsService {
    pub fn new(
        config: Arc<dyn BotConfigRepository>,
        repo: Arc<dyn SnapshotRepository>,
        analytics: Arc<dyn AnalyticsRepository>,
    ) -> Self {
        Self {
            config,
            repo,
            analytics,
        }
    }

    /// Config analytics d'une guild (jamais cachee — lue a chaque tick).
    async fn cfg(&self, guild_id: &str) -> Vec<BotGuildConfig> {
        self.config
            .get_config(guild_id, ANALYTICS_BOT)
            .await
            .unwrap_or_default()
    }
}

// Lecture de config : helpers partagés (sémantique de vérité unique — le
// `cfg_bool` local qui rejetait "yes" est remplacé par la référence).
use crate::sentinel::domain::entities::system::bot_config::{cfg_bool, cfg_i64, cfg_str};

fn module_enabled(entries: &[BotGuildConfig]) -> bool {
    cfg_bool(entries, "enabled", false)
}

#[async_trait]
impl ManageSnapshotsUseCase for ManageSnapshotsService {
    async fn snapshot_daily_all(&self) -> Result<JobReport, DomainError> {
        let guilds = self.repo.list_guild_ids().await?;
        let mut processed = 0;
        let mut skipped = 0;
        for guild_id in &guilds {
            let cfg = self.cfg(guild_id).await;
            if !module_enabled(&cfg) {
                skipped += 1;
                continue;
            }
            let track_voice = cfg_bool(&cfg, "track_voice_stats", true);
            let track_msg = cfg_bool(&cfg, "track_message_stats", true);
            let anchor_hour = cfg_i64(&cfg, "baseline_anchor_hour", 0).clamp(0, 23);
            if let Err(e) = self
                .repo
                .snapshot_daily(guild_id, track_msg, track_voice, anchor_hour)
                .await
            {
                tracing::warn!(error = %e, guild = %guild_id, "snapshot_daily echec");
                continue;
            }
            processed += 1;
        }
        Ok(JobReport::ok(processed, skipped))
    }

    async fn snapshot_hourly_all(&self) -> Result<JobReport, DomainError> {
        let guilds = self.repo.list_guild_ids().await?;
        let mut processed = 0;
        let mut skipped = 0;
        for guild_id in &guilds {
            let cfg = self.cfg(guild_id).await;
            if !module_enabled(&cfg) {
                skipped += 1;
                continue;
            }
            let track_msg = cfg_bool(&cfg, "track_message_stats", true);
            if let Err(e) = self.repo.snapshot_hourly(guild_id, track_msg).await {
                tracing::warn!(error = %e, guild = %guild_id, "snapshot_hourly echec");
                continue;
            }
            processed += 1;
        }
        Ok(JobReport::ok(processed, skipped))
    }

    async fn retention_cleanup_all(&self) -> Result<JobReport, DomainError> {
        let guilds = self.repo.list_guild_ids().await?;
        let mut processed = 0;
        let mut skipped = 0;
        for guild_id in &guilds {
            let cfg = self.cfg(guild_id).await;
            if !module_enabled(&cfg) {
                skipped += 1;
                continue;
            }
            // - data_retention_days  : daily_activity + analytics_daily_baseline (defaut 90j)
            // - hourly_retention_days : hourly_activity (defaut 30j)
            // 0 ou negatif = illimite, on ne purge pas cette dimension.
            let daily_retention = cfg_i64(&cfg, "data_retention_days", 90);
            let hourly_retention = cfg_i64(&cfg, "hourly_retention_days", 30);
            if daily_retention <= 0 && hourly_retention <= 0 {
                skipped += 1;
                continue;
            }
            if daily_retention > 0 {
                let r = daily_retention as i32;
                if let Err(e) = self.repo.cleanup_daily(guild_id, r).await {
                    tracing::warn!(error = %e, guild = %guild_id, "retention daily echec");
                }
                if let Err(e) = self.repo.cleanup_baseline(guild_id, r).await {
                    tracing::warn!(error = %e, guild = %guild_id, "retention baseline echec");
                }
            }
            if hourly_retention > 0 {
                if let Err(e) = self
                    .repo
                    .cleanup_hourly(guild_id, hourly_retention as i32)
                    .await
                {
                    tracing::warn!(error = %e, guild = %guild_id, "retention hourly echec");
                }
            }
            processed += 1;
        }
        Ok(JobReport::ok(processed, skipped))
    }

    async fn plan_top_publications(&self) -> Result<TopPublishPlan, DomainError> {
        let guilds = self.repo.list_guild_ids().await?;
        let now = chrono::Utc::now();
        let mut publications = Vec::new();
        let mut skipped = 0;

        for guild_id in &guilds {
            let cfg = self.cfg(guild_id).await;
            if !module_enabled(&cfg) || !cfg_bool(&cfg, "top_users_publish_enabled", false) {
                skipped += 1;
                continue;
            }
            let channel_id = match cfg_str(&cfg, "top_users_publish_channel_id") {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => {
                    skipped += 1;
                    continue;
                }
            };
            let interval_days = cfg_i64(&cfg, "top_users_publish_interval_days", 7);
            if let Some(s) = cfg_str(&cfg, LAST_PUBLISH_KEY) {
                if let Ok(last) = chrono::DateTime::parse_from_rfc3339(s) {
                    let elapsed = now.signed_duration_since(last.with_timezone(&chrono::Utc));
                    if elapsed < chrono::Duration::days(interval_days) {
                        skipped += 1;
                        continue;
                    }
                }
            }

            let count = cfg_i64(&cfg, "top_users_count", 10);
            let min_total = cfg_i64(&cfg, "low_activity_filter", 0).max(0);
            let top = match self
                .analytics
                .get_top_infractors(Some(guild_id), 30, count, min_total)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(error = %e, guild = %guild_id, "publish_top_users: get_top_infractors echec");
                    continue;
                }
            };

            let mut description = String::new();
            for (i, t) in top.iter().enumerate() {
                description.push_str(&format!(
                    "**{}.** <@{}> — {} infractions ({}w / {}m / {}b)\n",
                    i + 1,
                    t.user_id,
                    t.total_infractions,
                    t.warns,
                    t.mutes,
                    t.bans
                ));
            }
            if description.is_empty() {
                description.push_str("_Aucune infraction sur les 30 derniers jours._");
            }

            publications.push(TopPublication {
                guild_id: guild_id.clone(),
                channel_id,
                title: format!("Top {count} infracteurs (30j)"),
                description,
                color: TOP_EMBED_COLOR,
                published_at: now.to_rfc3339(),
            });
        }

        Ok(TopPublishPlan {
            publications,
            skipped,
        })
    }

    async fn mark_top_published(
        &self,
        guild_id: &str,
        published_at: &str,
    ) -> Result<(), DomainError> {
        self.config
            .set_config(guild_id, ANALYTICS_BOT, LAST_PUBLISH_KEY, published_at)
            .await
    }
}
