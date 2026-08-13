//! Jobs analytics declenches par le sentinel-worker (snapshots, cleanup,
//! publication Top users). Le metier vit dans le use case `ManageSnapshotsUseCase`
//! (archi hexagonale) : le worker se contente de tick et POST, ces handlers ne
//! font que RBAC/validation, appeler le use case et — pour Top users — poster
//! l'embed Discord.
//!
//! Endpoints :
//!   POST /api/analytics/snapshot/daily          → snapshot quotidien
//!   POST /api/analytics/snapshot/hourly         → snapshot horaire
//!   POST /api/analytics/retention-cleanup       → purge donnees > X jours
//!   POST /api/analytics/publish-top-users       → publie embed Top users
//!   GET  /api/analytics/export                  → export daily_activity

use axum::extract::{Query, State};
use axum::http::header;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::bootstrap::state::AuditState;
use platform_core::sentinel::domain::entities::audit::snapshot::JobReport;
use platform_core::sentinel::domain::errors::DomainError;

const ANALYTICS_BOT: &str = "analytics";

// ── Jobs ────────────────────────────────────────────────────────────────

/// POST /api/analytics/snapshot/daily
pub async fn snapshot_daily_all(
    State(state): State<AuditState>,
) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(state.snapshots_uc.snapshot_daily_all().await?))
}

/// POST /api/analytics/snapshot/hourly
pub async fn snapshot_hourly_all(
    State(state): State<AuditState>,
) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(state.snapshots_uc.snapshot_hourly_all().await?))
}

/// POST /api/analytics/retention-cleanup
pub async fn retention_cleanup_all(
    State(state): State<AuditState>,
) -> Result<Json<JobReport>, ApiError> {
    Ok(Json(state.snapshots_uc.retention_cleanup_all().await?))
}

/// POST /api/analytics/publish-top-users
///
/// Le use case calcule les publications dues (config par guild, intervalle,
/// top infracteurs). Ce handler poste l'embed Discord (concern inbound) et
/// persiste l'horodatage via le use case apres un post reussi.
pub async fn publish_top_users_all(
    State(state): State<AuditState>,
) -> Result<Json<JobReport>, ApiError> {
    let plan = state.snapshots_uc.plan_top_publications().await?;
    let mut processed = 0;

    for pub_ in &plan.publications {
        let embed = serde_json::json!({
            "title": pub_.title,
            "description": pub_.description,
            "color": pub_.color,
            "timestamp": pub_.published_at,
        });

        // Validation du salon, absence de token et statut HTTP : traites par
        // l'adaptateur. On n'avance `processed` et on n'horodate qu'apres un
        // envoi reussi, sinon une publication ratee serait consideree faite.
        if let Err(e) = state
            .discord_api
            .send_channel_embed(&pub_.channel_id, embed)
            .await
        {
            tracing::warn!(error = %e, guild = %pub_.guild_id, "publish_top_users: publication echouee");
            continue;
        }

        if let Err(e) = state
            .snapshots_uc
            .mark_top_published(&pub_.guild_id, &pub_.published_at)
            .await
        {
            tracing::warn!(error = %e, guild = %pub_.guild_id, "publish_top_users: persist last echec");
        }
        processed += 1;
    }

    Ok(Json(JobReport::ok(processed, plan.skipped)))
}

// ── Export ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ExportQuery {
    pub guild_id: String,
    pub days: Option<i32>,
    /// "json" | "csv". Si absent, fallback sur la cle `export_format` du guild.
    pub format: Option<String>,
}

/// GET /api/analytics/export?guild_id=...&days=N&format=json|csv
pub async fn export_analytics(
    State(state): State<AuditState>,
    Query(params): Query<ExportQuery>,
) -> Result<Response, ApiError> {
    if params.guild_id.is_empty() {
        return Err(ApiError(DomainError::ValidationError(
            "guild_id requis".into(),
        )));
    }
    // IDOR : export cross-serveur de l'activite (messages/vocal/infractions).
    let days =
        crate::sentinel::adapters::inbound::http::helpers::normalize_in(params.days, 30, 1, 365);

    let format = match params.format {
        Some(f) if !f.is_empty() => f,
        _ => read_export_format(&state, &params.guild_id).await,
    };
    let format = format.to_lowercase();

    let activities = state
        .daily_activity_repo
        .get_activity(Some(&params.guild_id), days)
        .await?;

    match format.as_str() {
        "csv" => {
            let mut out = String::from("day,messages,voice_minutes,active_members,new_members,leaves,infractions,warns,mutes,bans\n");
            for a in &activities {
                out.push_str(&format!(
                    "{},{},{},{},{},{},{},{},{},{}\n",
                    a.day,
                    a.messages,
                    a.voice_minutes,
                    a.active_members,
                    a.new_members,
                    a.leaves,
                    a.infractions,
                    a.warns,
                    a.mutes,
                    a.bans
                ));
            }
            Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=\"analytics.csv\"",
                    ),
                ],
                out,
            )
                .into_response())
        }
        _ => {
            #[derive(Serialize)]
            struct Row {
                day: String,
                messages: i64,
                voice_minutes: i64,
                active_members: i32,
                new_members: i32,
                leaves: i32,
                infractions: i32,
                warns: i32,
                mutes: i32,
                bans: i32,
            }
            let rows: Vec<Row> = activities
                .into_iter()
                .map(|a| Row {
                    day: a.day.to_string(),
                    messages: a.messages,
                    voice_minutes: a.voice_minutes,
                    active_members: a.active_members,
                    new_members: a.new_members,
                    leaves: a.leaves,
                    infractions: a.infractions,
                    warns: a.warns,
                    mutes: a.mutes,
                    bans: a.bans,
                })
                .collect();
            Ok(Json(rows).into_response())
        }
    }
}

/// Lit la cle `export_format` du guild (fallback "json"). Concern handler : ne
/// touche pas au SQL (passe par le repo de config).
async fn read_export_format(state: &AuditState, guild_id: &str) -> String {
    state
        .bot_config_repo
        .get_config(guild_id, ANALYTICS_BOT)
        .await
        .ok()
        .and_then(|cfgs| {
            cfgs.into_iter()
                .find(|c| c.config_key == "export_format")
                .map(|c| c.config_value)
        })
        .unwrap_or_else(|| "json".into())
}
