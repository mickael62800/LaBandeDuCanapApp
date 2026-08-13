//! Port entrant du copilote de moderation (cas d'usage lecture seule).

use async_trait::async_trait;

use crate::sentinel::domain::entities::moderation::copilot::MemberModerationContext;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ModerationCopilotUseCase: Send + Sync {
    /// Assemble le contexte de moderation d'un membre + une suggestion de
    /// sanction proportionnee (consultative). `lookback_days` borne la fenetre
    /// d'agregation ; `min_precedents` le seuil de confiance de la jurisprudence.
    async fn get_member_context(
        &self,
        guild_id: &str,
        user_id: &str,
        lookback_days: i64,
        min_precedents: u32,
    ) -> Result<MemberModerationContext, DomainError>;
}
