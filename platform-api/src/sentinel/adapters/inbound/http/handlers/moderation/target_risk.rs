//! Handler HTTP — evaluation server-side du risque d'une cible de moderation.
//!
//! `POST /api/moderation/{guild_id}/assess-target-risk`
//! Le bot envoie les FAITS Discord de la cible (age du compte, is_bot,
//! has_mod_perms) ; le use case applique le SEUIL serveur + la POLITIQUE et
//! renvoie `{risky, reason}`. Aucune regle metier ici : authz + mapping.
//! RBAC : Moderateur+ (les appels bot `AuthKind::Internal` passent en
//! pass-through via `check_role_for_guild`).

use axum::extract::State;
use axum::Json;

use crate::sentinel::adapters::inbound::http::dto::moderation::target_risk::{
    AssessTargetRiskRequestDto, TargetRiskDecisionDto,
};
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use crate::sentinel::bootstrap::state::ModerationState;
use platform_core::sentinel::ports::inbound::moderation::assess_target_risk::AssessTargetRiskCommand;

/// POST /api/moderation/{guild_id}/assess-target-risk
pub async fn assess_target_risk(
    State(state): State<ModerationState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<AssessTargetRiskRequestDto>,
) -> Result<Json<TargetRiskDecisionDto>, ApiError> {
    let decision = state
        .assess_target_risk_uc
        .assess(AssessTargetRiskCommand {
            guild_id,
            account_age_days: dto.account_age_days,
            is_bot: dto.is_bot,
            has_mod_perms: dto.has_mod_perms,
        })
        .await?;

    Ok(Json(decision.into()))
}
