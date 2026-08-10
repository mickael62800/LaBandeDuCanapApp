//! Regles d'alerte de supervision (table `alert_rules`).
//!
//! Portees depuis `sentinel-api` : elles pilotent le dispatcher d'alertes de la
//! machine, pas la moderation Discord. Le handler ne fait que
//! authentification + mapping DTO ; les invariants et le SQL vivent derriere le
//! use case (`ManageAlertRules` + `PgAlertRuleRepository`).

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use ops_core::domain::entities::alert_rule::{AlertRule, AlertRuleUpdate};
use serde::{Deserialize, Serialize};

use crate::{authorize, ApiError, AppState};

#[derive(Serialize)]
pub struct AlertRuleDto {
    pub id: String,
    pub label: String,
    pub metric: String,
    pub comparator: String,
    pub threshold: Option<f64>,
    pub enabled: bool,
    pub severity: String,
    pub cooldown_secs: i32,
}

impl From<AlertRule> for AlertRuleDto {
    fn from(r: AlertRule) -> Self {
        Self {
            id: r.id,
            label: r.label,
            metric: r.metric,
            comparator: r.comparator,
            threshold: r.threshold,
            enabled: r.enabled,
            severity: r.severity,
            cooldown_secs: r.cooldown_secs,
        }
    }
}

/// GET /alert-rules — toutes les regles, actives ou non.
pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AlertRuleDto>>, ApiError> {
    authorize(&headers, &state.config)?;
    let rules = state.alert_rules_uc.list().await?;
    Ok(Json(rules.into_iter().map(Into::into).collect()))
}

#[derive(Deserialize)]
pub struct UpdateAlertRuleDto {
    pub enabled: Option<bool>,
    pub threshold: Option<f64>,
    pub severity: Option<String>,
    pub cooldown_secs: Option<i32>,
}

/// PATCH /alert-rules/{id} — champs editables uniquement.
///
/// `metric`, `comparator` et `label` sont fixes : ils definissent la semantique
/// de la regle, et les rendre modifiables permettrait de renommer une alerte
/// sans changer ce qu'elle mesure.
pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(dto): Json<UpdateAlertRuleDto>,
) -> Result<Json<AlertRuleDto>, ApiError> {
    authorize(&headers, &state.config)?;
    let update = AlertRuleUpdate {
        enabled: dto.enabled,
        threshold: dto.threshold,
        severity: dto.severity,
        cooldown_secs: dto.cooldown_secs,
    };
    let rule = state.alert_rules_uc.update(&id, update).await?;
    Ok(Json(rule.into()))
}
