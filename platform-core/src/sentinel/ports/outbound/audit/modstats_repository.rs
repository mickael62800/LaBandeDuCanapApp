use async_trait::async_trait;

use crate::sentinel::domain::entities::moderation::modstats::ModeratorBreakdown;
use crate::sentinel::domain::entities::moderation::modstats::ModstatsTrendDay;
use crate::sentinel::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct ModeratorStat {
    pub moderator_id: String,
    pub moderator_name: String,
    pub action_count: i64,
}

#[async_trait]
pub trait ModstatsRepository: Send + Sync {
    async fn top_moderators(
        &self,
        guild_id: &str,
        days: i32,
        limit: i64,
    ) -> Result<Vec<ModeratorStat>, DomainError>;

    /// Breakdown par moderateur (warns/mutes/bans/kicks) sur `days` jours, top `limit`.
    /// Default : vide (pour les stubs de test).
    async fn breakdown(
        &self,
        _guild_id: &str,
        _days: i32,
        _limit: i64,
    ) -> Result<Vec<ModeratorBreakdown>, DomainError> {
        Ok(vec![])
    }

    /// Comptes quotidiens d'actions sur `days` jours (un point par jour).
    /// Default : vide (pour les stubs de test).
    async fn daily_trend(
        &self,
        _guild_id: &str,
        _days: i32,
    ) -> Result<Vec<ModstatsTrendDay>, DomainError> {
        Ok(vec![])
    }
}
