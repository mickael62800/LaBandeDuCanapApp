//! Port outbound du classement mensuel : regroupe les requetes SQL
//! d'agregation (deltas d'XP par membre) et de gestion des baselines
//! (snapshots mensuels). L'assemblage du classement est de la logique metier
//! et vit dans `ManageMonthlyRankingService`.

use async_trait::async_trait;

use crate::sentinel::domain::entities::community::monthly_ranking::RankingRow;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait MonthlyRankingRepository: Send + Sync {
    /// Deltas d'XP (texte / vocal) par membre pour la periode `baseline_period_ym`.
    /// Delta = XP cumulee actuelle - baseline du snapshot. Absence de baseline =>
    /// fallback cumul total (COALESCE 0). Les membres portant un des
    /// `excluded_roles` (JSONB array d'IDs) sont ecartes ; array vide = aucun.
    async fn ranking_deltas(
        &self,
        guild_id: &str,
        baseline_period_ym: &str,
        excluded_roles: &[String],
    ) -> Result<Vec<RankingRow>, DomainError>;

    /// Une baseline existe-t-elle pour `(guild, period_ym)` ?
    async fn has_baseline(&self, guild_id: &str, period_ym: &str) -> Result<bool, DomainError>;

    /// Tous les `guild_id` connus (tries par nom).
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError>;

    /// `bool_or(partial)` de la baseline `(guild, period_ym)` : `Some(false)` =>
    /// baseline COMPLETE (publiable), `Some(true)` => partielle, `None` => absente.
    async fn baseline_partial_flag(
        &self,
        guild_id: &str,
        period_ym: &str,
    ) -> Result<Option<bool>, DomainError>;

    /// Existe-t-il une baseline anterieure a `period_ym` pour cette guild ?
    async fn has_prior_baseline(
        &self,
        guild_id: &str,
        period_ym: &str,
    ) -> Result<bool, DomainError>;

    /// Pose la baseline du mois courant (idempotent, ON CONFLICT DO NOTHING) a
    /// partir de l'XP cumulee actuelle de `user_levels`.
    async fn insert_baseline(
        &self,
        guild_id: &str,
        period_ym: &str,
        partial: bool,
    ) -> Result<(), DomainError>;
}
