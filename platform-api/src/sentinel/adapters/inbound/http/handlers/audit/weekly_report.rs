use axum::extract::Path;
use axum::extract::State;
use axum::Json;

use crate::sentinel::adapters::inbound::http::dto::audit::weekly_report::WeeklyReportDto;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::bootstrap::state::AuditState;

/// GET /api/audit-weekly-report/{guild_id} — rapport d'activite hebdomadaire
/// agrege server-side depuis les events d'audit persistes (fenetre 7 jours).
/// Remplace l'ancien `WeeklyTracker` RAM du bot ; le bot ne fait plus que rendre
/// l'embed a partir de ces donnees.
pub async fn get_weekly_report(
    State(state): State<AuditState>,
    Path(guild_id): Path<String>,
) -> Result<Json<WeeklyReportDto>, ApiError> {
    let report = state.weekly_report_uc.get(&guild_id).await?;
    Ok(Json(report.into()))
}
