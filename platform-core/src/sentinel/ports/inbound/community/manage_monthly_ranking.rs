//! Port inbound (use case) du classement mensuel d'activite.
//!
//! Le handler HTTP ne fait que parser/RBAC/poster : l'assemblage du classement
//! (deltas, tops, blocs), les gates de publication et la pose des baselines
//! vivent dans `ManageMonthlyRankingService`.

use async_trait::async_trait;

use crate::sentinel::domain::entities::community::monthly_ranking::{
    MonthlyPublishPlan, MonthlyRankingData,
};
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ManageMonthlyRankingUseCase: Send + Sync {
    /// Publication FORCEE d'un mois (`"actuel"` par defaut | `"precedent"`) :
    /// bypass toutes les gates, ne poste PAS sur Discord, renvoie les tops.
    async fn force_ranking(
        &self,
        guild_id: &str,
        mois: Option<String>,
    ) -> Result<MonthlyRankingData, DomainError>;

    /// Passe toutes les guilds : applique les gates (module + feature actifs,
    /// baseline du mois deja posee, mois precedent complet), pose la baseline du
    /// mois courant, et renvoie le plan des classements a poster sur Discord.
    async fn plan_and_baseline(&self) -> Result<MonthlyPublishPlan, DomainError>;

    /// Marque un mois publie pour une guild (memorise la derniere periode).
    async fn mark_published(&self, guild_id: &str, period: &str) -> Result<(), DomainError>;
}
