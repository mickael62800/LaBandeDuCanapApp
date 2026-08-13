//! Use case du classement mensuel : assemblage des tops (texte / vocal /
//! global) a partir des deltas d'XP, gates de publication et pose des
//! baselines. Toute la regle metier vit ici ; le SQL vit dans
//! `MonthlyRankingRepository`, la config passe par `BotConfigRepository`.

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::entities::community::monthly_ranking::{
    build_ranking_block, current_and_prev_periods, month_label_fr, top_entries, MonthlyPublishItem,
    MonthlyPublishPlan, MonthlyRankingData, RankingRow,
};
use crate::sentinel::domain::entities::system::bot_config::BotGuildConfig;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_monthly_ranking::ManageMonthlyRankingUseCase;
use crate::sentinel::ports::outbound::community::monthly_ranking_repository::MonthlyRankingRepository;
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;

use crate::sentinel::domain::entities::system::bot_names::PROGRESSION_BOT;
const LAST_PERIOD_KEY: &str = "monthly_ranking_last_period";

pub struct ManageMonthlyRankingService {
    config: Arc<dyn BotConfigRepository>,
    repo: Arc<dyn MonthlyRankingRepository>,
}

impl ManageMonthlyRankingService {
    pub fn new(
        config: Arc<dyn BotConfigRepository>,
        repo: Arc<dyn MonthlyRankingRepository>,
    ) -> Self {
        Self { config, repo }
    }

    async fn load_config(&self, guild_id: &str) -> Vec<BotGuildConfig> {
        self.config
            .get_config(guild_id, PROGRESSION_BOT)
            .await
            .unwrap_or_default()
    }

    /// Deltas d'XP pour la periode, en appliquant les roles exclus configures.
    async fn deltas(
        &self,
        cfg: &[BotGuildConfig],
        guild_id: &str,
        baseline_period_ym: &str,
    ) -> Result<Vec<RankingRow>, DomainError> {
        let excluded_roles = parse_csv(cfg_str(cfg, "monthly_ranking_excluded_roles"));
        self.repo
            .ranking_deltas(guild_id, baseline_period_ym, &excluded_roles)
            .await
    }
}

use crate::sentinel::domain::entities::system::bot_config::{cfg_bool, cfg_str};

fn cfg_top(entries: &[BotGuildConfig]) -> usize {
    cfg_str(entries, "monthly_ranking_top_count")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(10)
        .clamp(1, 25) as usize
}
fn parse_csv(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[async_trait]
impl ManageMonthlyRankingUseCase for ManageMonthlyRankingService {
    async fn force_ranking(
        &self,
        guild_id: &str,
        mois: Option<String>,
    ) -> Result<MonthlyRankingData, DomainError> {
        let (this_period, prev_period) = current_and_prev_periods(chrono::Utc::now());
        // period_ym = mois affiche ; baseline_period_ym = snapshot de reference
        // (identiques ici : on montre le delta du mois demande).
        let period_ym = match mois.as_deref() {
            Some("precedent") => prev_period,
            _ => this_period,
        };

        let cfg = self.load_config(guild_id).await;

        // Fallback cumul total : si pas de baseline pour la periode, la SQL
        // retombe deja sur l'XP cumulee (COALESCE 0). On le signale via `note`.
        let note = if self.repo.has_baseline(guild_id, &period_ym).await? {
            None
        } else {
            Some("(cumul total \u{2014} pas de baseline ce mois)".to_string())
        };

        let rows = self.deltas(&cfg, guild_id, &period_ym).await?;
        let top = cfg_top(&cfg);

        Ok(MonthlyRankingData {
            period_label: month_label_fr(&period_ym),
            note,
            text: top_entries(&rows, top, |t, _| t),
            voice: top_entries(&rows, top, |_, v| v),
            global: top_entries(&rows, top, |t, v| t + v),
        })
    }

    async fn plan_and_baseline(&self) -> Result<MonthlyPublishPlan, DomainError> {
        let (this_period, prev_period) = current_and_prev_periods(chrono::Utc::now());
        let guilds = self.repo.list_guild_ids().await?;

        let mut plan = MonthlyPublishPlan::default();

        for guild_id in &guilds {
            let cfg = self.load_config(guild_id).await;

            // Module + feature actifs ?
            if !cfg_bool(&cfg, "enabled", false)
                || !cfg_bool(&cfg, "monthly_ranking_enabled", false)
            {
                plan.skipped += 1;
                continue;
            }

            // Baseline du mois courant deja posee -> rien a faire ce mois-ci.
            if self.repo.has_baseline(guild_id, &this_period).await? {
                plan.skipped += 1;
                continue;
            }

            // Le mois precedent a-t-il une baseline COMPLETE (publiable) ?
            let prev_complete = matches!(
                self.repo
                    .baseline_partial_flag(guild_id, &prev_period)
                    .await?,
                Some(false)
            );
            if prev_complete {
                if let Some(channel_id) =
                    cfg_str(&cfg, "monthly_ranking_channel_id").filter(|s| !s.is_empty())
                {
                    let top = cfg_top(&cfg);
                    let rows = self.deltas(&cfg, guild_id, &prev_period).await?;
                    plan.publications.push(MonthlyPublishItem {
                        guild_id: guild_id.clone(),
                        channel_id: channel_id.to_string(),
                        period: prev_period.clone(),
                        period_label: month_label_fr(&prev_period),
                        text_block: build_ranking_block(
                            rows.iter().map(|r| (r.user_id.clone(), r.d_text)).collect(),
                            top,
                        ),
                        voice_block: build_ranking_block(
                            rows.iter()
                                .map(|r| (r.user_id.clone(), r.d_voice))
                                .collect(),
                            top,
                        ),
                        global_block: build_ranking_block(
                            rows.iter()
                                .map(|r| (r.user_id.clone(), r.d_text + r.d_voice))
                                .collect(),
                            top,
                        ),
                    });
                }
            }

            // `partial` fonde sur la CONTINUITE : une baseline n'est partielle
            // (jamais publiee) que si c'est la toute premiere de ce serveur.
            let has_prior = self.repo.has_prior_baseline(guild_id, &this_period).await?;
            self.repo
                .insert_baseline(guild_id, &this_period, !has_prior)
                .await?;
            plan.baselined += 1;
        }

        Ok(plan)
    }

    async fn mark_published(&self, guild_id: &str, period: &str) -> Result<(), DomainError> {
        self.config
            .set_config(guild_id, PROGRESSION_BOT, LAST_PERIOD_KEY, period)
            .await
    }
}
