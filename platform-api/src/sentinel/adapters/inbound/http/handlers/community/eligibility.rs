//! Eligibilite Community (HTTP) : adaptateur ENTRANT mince. La DECISION
//! (prerequis de role, regles de parrainage + seuils) vit dans
//! `CheckEligibilityUseCase` ; ici : parse + RBAC + map. Le bot fournit les
//! donnees Discord (roles actuels, dates de join) et applique la decision.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use crate::sentinel::bootstrap::state::CommunityState;
use platform_core::sentinel::ports::inbound::community::check_eligibility::{
    CheckRoleEligibilityCommand, ValidateSponsorshipCommand,
};

/// Reponse commune : decision d'eligibilite (autorise / refus + raison).
#[derive(Debug, Serialize)]
pub struct EligibilityDto {
    pub allowed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RoleEligibilityBody {
    /// Role Discord vise.
    pub role_id: u64,
    /// Roles Discord actuels du membre.
    #[serde(default)]
    pub user_roles: Vec<u64>,
    /// Timestamp unix (s) du join Discord. Absent => 0 jour d'anciennete.
    #[serde(default)]
    pub joined_at_unix: Option<i64>,
}

/// POST /api/community/eligibility/{guild_id}/role — decide de l'eligibilite au role.
pub async fn check_role_eligibility(
    State(state): State<CommunityState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(body): Json<RoleEligibilityBody>,
) -> Result<Json<EligibilityDto>, ApiError> {
    // Decision d'attribution de role : operation du BOT (Bearer API_KEY ->
    // Internal, bypass). Sans cette garde, un appelant web sonderait les regles
    // d'un serveur arbitraire.

    let decision = state
        .eligibility_uc
        .check_role_eligibility(CheckRoleEligibilityCommand {
            guild_id,
            role_id: body.role_id,
            user_roles: body.user_roles,
            joined_at_unix: body.joined_at_unix,
        })
        .await?;

    Ok(Json(EligibilityDto {
        allowed: decision.allowed,
        reason: decision.reason,
    }))
}

#[derive(Debug, Deserialize)]
pub struct SponsorshipEligibilityBody {
    pub sponsor_id: u64,
    pub sponsored_id: u64,
    /// Join Discord du parrain (unix s). Absent => echoue le min.
    #[serde(default)]
    pub sponsor_joined_at_unix: Option<i64>,
    /// Join Discord du filleul (unix s). Absent => echoue le max.
    #[serde(default)]
    pub sponsored_joined_at_unix: Option<i64>,
}

/// POST /api/community/eligibility/{guild_id}/sponsorship — valide un parrainage.
pub async fn validate_sponsorship(
    State(state): State<CommunityState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(body): Json<SponsorshipEligibilityBody>,
) -> Result<Json<EligibilityDto>, ApiError> {
    let decision = state
        .eligibility_uc
        .validate_sponsorship(ValidateSponsorshipCommand {
            guild_id,
            sponsor_id: body.sponsor_id,
            sponsored_id: body.sponsored_id,
            sponsor_joined_at_unix: body.sponsor_joined_at_unix,
            sponsored_joined_at_unix: body.sponsored_joined_at_unix,
        })
        .await?;

    Ok(Json(EligibilityDto {
        allowed: decision.allowed,
        reason: decision.reason,
    }))
}
