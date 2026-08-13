use std::sync::Arc;

use async_trait::async_trait;

use tracing::warn;

use crate::sentinel::domain::entities::audit::dashboard_stats::DashboardStats;
use crate::sentinel::domain::entities::audit::user_stats::GuildStatsOverview;
use crate::sentinel::domain::entities::audit::user_stats::GuildVoiceStats;
use crate::sentinel::domain::entities::audit::user_stats::UserStats;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use crate::sentinel::ports::inbound::audit::manage_stats::RecordMessagesCommand;
use crate::sentinel::ports::inbound::audit::manage_stats::RecordVoiceCommand;
use crate::sentinel::ports::inbound::moderation::manage_infractions::InfractionFilters;
use crate::sentinel::ports::outbound::audit::stats_repository::StatsRepository;
use crate::sentinel::ports::outbound::moderation::infraction_repository::InfractionRepository;
use crate::sentinel::ports::outbound::system::cache::CachePort;
use crate::sentinel::ports::outbound::system::cache_helpers::cached_json;

const OVERVIEW_TTL: u64 = 60; // 1 minute

pub struct ManageStatsService {
    stats_repo: Arc<dyn StatsRepository>,
    infraction_repo: Arc<dyn InfractionRepository>,
    cache: Arc<dyn CachePort>,
}

impl ManageStatsService {
    pub fn new(
        stats_repo: Arc<dyn StatsRepository>,
        infraction_repo: Arc<dyn InfractionRepository>,
        cache: Arc<dyn CachePort>,
    ) -> Self {
        Self {
            stats_repo,
            infraction_repo,
            cache,
        }
    }
}

#[async_trait]
impl ManageStatsUseCase for ManageStatsService {
    async fn record_messages(&self, cmd: RecordMessagesCommand) -> Result<(), DomainError> {
        self.stats_repo
            .increment_messages(&cmd.guild_id, &cmd.user_id, &cmd.username, cmd.count)
            .await?;

        // Invalidate caches
        let overview_key = format!("stats:overview:{}", cmd.guild_id);
        if let Err(e) = self.cache.invalidate(&overview_key).await {
            warn!(error = %e, key = %overview_key, "Echec invalidation cache stats overview");
        }

        Ok(())
    }

    async fn record_voice(&self, cmd: RecordVoiceCommand) -> Result<(), DomainError> {
        self.stats_repo
            .add_voice_seconds(&cmd.guild_id, &cmd.user_id, &cmd.username, cmd.seconds)
            .await?;

        // Enregistrer la session vocale detaillee (par salon)
        if !cmd.channel_id.is_empty() {
            self.stats_repo
                .save_voice_session(
                    &cmd.guild_id,
                    &cmd.user_id,
                    &cmd.username,
                    &cmd.channel_id,
                    &cmd.channel_name,
                    cmd.seconds,
                )
                .await
                .inspect_err(|e| warn!(error = %e, guild_id = %cmd.guild_id, "Echec sauvegarde session vocale"))
                .ok();
        }

        let overview_key = format!("stats:overview:{}", cmd.guild_id);
        if let Err(e) = self.cache.invalidate(&overview_key).await {
            warn!(error = %e, key = %overview_key, "Echec invalidation cache stats overview");
        }

        // Invalider les caches voice_stats pour les periodes courantes
        for days in [7, 30, 90] {
            let key = format!("voice_stats:{}:{days}:20", cmd.guild_id);
            if let Err(e) = self.cache.invalidate(&key).await {
                warn!(error = %e, key = %key, "Echec invalidation cache voice_stats");
            }
        }

        Ok(())
    }

    async fn get_user_stats(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserStats>, DomainError> {
        self.stats_repo.find_by_user(guild_id, user_id).await
    }

    async fn get_guild_overview(&self, guild_id: &str) -> Result<GuildStatsOverview, DomainError> {
        let cache_key = format!("stats:overview:{guild_id}");
        cached_json(&self.cache, &cache_key, OVERVIEW_TTL, || async {
            // Fetch stats from DB
            let members = self.stats_repo.find_by_guild(guild_id, 100).await?;

            let total_messages: u64 = members.iter().map(|m| m.message_count).sum();
            let total_voice_seconds: u64 = members.iter().map(|m| m.voice_seconds).sum();
            let active_members = members.len() as u64;

            // Fetch infractions
            let filters = InfractionFilters {
                user_id: None,
                action: None,
                limit: 10000,
                offset: 0,
            };
            let infractions = self
                .infraction_repo
                .find_by_guild(guild_id, &filters)
                .await
                .unwrap_or_else(|e| {
                    warn!(error = %e, guild_id, "Echec chargement infractions pour stats overview");
                    vec![]
                });

            let total_warns = infractions
                .iter()
                .filter(|i| i.action.as_str() == "warn")
                .count() as u64;
            let total_mutes = infractions
                .iter()
                .filter(|i| i.action.as_str() == "mute")
                .count() as u64;
            let total_bans = infractions
                .iter()
                .filter(|i| i.action.as_str() == "ban")
                .count() as u64;

            let top_members: Vec<UserStats> = members.into_iter().take(10).collect();

            Ok(GuildStatsOverview {
                guild_id: guild_id.to_string().into(),
                total_messages,
                total_voice_seconds,
                active_members,
                total_infractions: infractions.len() as u64,
                total_warns,
                total_mutes,
                total_bans,
                top_members,
            })
        })
        .await
    }

    async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<UserStats>, DomainError> {
        self.stats_repo.find_by_guild(guild_id, limit).await
    }

    async fn get_dashboard_stats(&self) -> Result<DashboardStats, DomainError> {
        let total_servers = self.stats_repo.count_distinct_guilds().await.unwrap_or(0) as u32;
        let total_users = self.stats_repo.count_distinct_users().await.unwrap_or(0) as u32;
        let infractions_today = self.infraction_repo.count_today().await.unwrap_or(0) as u32;

        // Disponibilite de la base de Sentinel, constatee en lisant ses propres
        // tables. La sante des services (bots, workers, Redis) n'est plus ici :
        // elle appartient a l'exploitation, et c'est l'adaptateur HTTP qui
        // compose les deux pour le tableau de bord.
        let postgres_online = self.stats_repo.count_distinct_guilds().await.is_ok();

        Ok(DashboardStats {
            total_servers,
            total_users,
            messages_today: 0,
            infractions_today,
            postgres_online,
        })
    }

    async fn get_guild_voice_stats(
        &self,
        guild_id: &str,
        days: u32,
        limit: u32,
    ) -> Result<GuildVoiceStats, DomainError> {
        let cache_key = format!("voice_stats:{guild_id}:{days}:{limit}");
        cached_json(&self.cache, &cache_key, OVERVIEW_TTL, || async {
            let channels = self
                .stats_repo
                .get_guild_voice_stats(guild_id, days, limit)
                .await?;
            let unique_users = self
                .stats_repo
                .count_unique_voice_users(guild_id, days)
                .await?;

            let total_channels = channels.len() as i64;
            let total_sessions: i64 = channels.iter().map(|c| c.total_sessions).sum();
            let total_duration_secs: i64 = channels.iter().map(|c| c.total_duration_secs).sum();
            let avg_session_secs = if total_sessions > 0 {
                total_duration_secs / total_sessions
            } else {
                0
            };
            let temp_channels = channels.iter().filter(|c| c.is_temporary).count() as i64;
            let perm_channels = channels.iter().filter(|c| !c.is_temporary).count() as i64;

            Ok(GuildVoiceStats {
                total_channels,
                total_sessions,
                total_duration_secs,
                unique_users,
                avg_session_secs,
                temp_channels,
                perm_channels,
                channels,
            })
        })
        .await
    }
}

#[cfg(test)]
#[path = "tests/manage_stats.rs"]
mod tests;
