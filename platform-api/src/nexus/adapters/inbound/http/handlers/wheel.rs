//! Handler POST /api/wheel/{guild_id}/{user_id}/spin

use axum::extract::Path;
use axum::extract::State;
use axum::Json;
use platform_core::nexus::ports::inbound::play_wheel::PlayWheelCommand;
use platform_core::nexus::ports::inbound::play_wheel::PlayWheelResult;
use serde::Deserialize;
use serde::Serialize;

use super::ApiError;
use crate::nexus::bootstrap::AppState;

#[derive(Debug, Deserialize)]
pub struct WheelSpinDto {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct WheelSpinResponseDto {
    pub spin_id: String,
    pub case_key: String,
    pub case_label: String,
    pub payout: i64,
    pub balance_after: i64,
    pub is_memorable: bool,
}

impl From<PlayWheelResult> for WheelSpinResponseDto {
    fn from(r: PlayWheelResult) -> Self {
        Self {
            spin_id: r.spin.id.to_string(),
            case_key: r.spin.case_key,
            case_label: r.spin.case_label,
            payout: r.spin.payout,
            balance_after: r.balance_after,
            is_memorable: r.is_memorable,
        }
    }
}

pub async fn spin(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<WheelSpinDto>,
) -> Result<Json<WheelSpinResponseDto>, ApiError> {
    let result = state
        .play_wheel
        .spin(PlayWheelCommand {
            guild_id,
            user_id,
            username: dto.username,
        })
        .await?;
    Ok(Json(WheelSpinResponseDto::from(result)))
}

#[derive(Debug, Serialize)]
pub struct WheelStatusDto {
    /// Le joueur peut-il encore tirer aujourd'hui ?
    pub can_spin: bool,
}

/// GET /api/wheel/{guild_id}/{user_id}/status
///
/// Lecture seule : permet a une interface de fermer son bouton avant tout
/// clic. La regle reste arbitree par `spin` — deux clics simultanes passent
/// tous deux ce controle, seul le claim atomique tranche.
pub async fn status(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<WheelStatusDto>, ApiError> {
    let can_spin = state.play_wheel.can_spin(&guild_id, &user_id).await?;
    Ok(Json(WheelStatusDto { can_spin }))
}

// ── Cases de la roue (edition par serveur) ──

#[derive(Debug, Serialize, Deserialize)]
pub struct WheelCaseDto {
    pub key: String,
    pub label: String,
    pub payout: i64,
    pub weight: u32,
}

#[derive(Debug, Serialize)]
pub struct WheelCasesDto {
    pub cases: Vec<WheelCaseDto>,
    /// `false` = ce sont les cases historiques, faute de personnalisation.
    pub customized: bool,
}

impl From<platform_core::nexus::ports::inbound::wheel_cases::WheelCases> for WheelCasesDto {
    fn from(w: platform_core::nexus::ports::inbound::wheel_cases::WheelCases) -> Self {
        Self {
            cases: w
                .cases
                .into_iter()
                .map(|c| WheelCaseDto {
                    key: c.key,
                    label: c.label,
                    payout: c.payout,
                    weight: c.weight,
                })
                .collect(),
            customized: w.customized,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ReplaceWheelCasesDto {
    /// Liste VIDE = revenir a la roue historique.
    pub cases: Vec<WheelCaseDto>,
}

/// GET /api/wheel/{guild_id}/cases
pub async fn list_cases(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<WheelCasesDto>, ApiError> {
    Ok(Json(state.wheel_cases.list(&guild_id).await?.into()))
}

/// PUT /api/wheel/{guild_id}/cases — remplace INTEGRALEMENT la roue.
pub async fn replace_cases(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<ReplaceWheelCasesDto>,
) -> Result<Json<WheelCasesDto>, ApiError> {
    let cases = dto
        .cases
        .into_iter()
        .map(
            |c| platform_core::nexus::domain::entities::wheel::WheelCaseData {
                key: c.key,
                label: c.label,
                payout: c.payout,
                weight: c.weight,
            },
        )
        .collect();
    Ok(Json(
        state.wheel_cases.replace(&guild_id, cases).await?.into(),
    ))
}
// Handlers HTTP de la Roue du Destin. L'édition remplace toute la liste des
// cases ; une liste vide revient explicitement aux cases par défaut.
