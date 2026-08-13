//! Regles d'alerte de supervision (table `alert_rules`).
//!
//! Portees depuis `sentinel-api` : elles pilotent le dispatcher d'alertes de la
//! machine, pas la moderation Discord. Le handler ne fait que
//! authentification + mapping DTO ; les invariants et le SQL vivent derriere le
//! use case (`ManageAlertRules` + `PgAlertRuleRepository`).

use axum::extract::{Path, State};
use axum::Json;
use platform_core::ops::domain::entities::alert_rule::{AlertRule, AlertRuleUpdate};
use serde::{Deserialize, Serialize};

use crate::ops::{ApiError, AppState};

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
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<AlertRuleDto>>, ApiError> {
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
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(dto): Json<UpdateAlertRuleDto>,
) -> Result<Json<AlertRuleDto>, ApiError> {
    let actor = crate::ops::handlers::security::actor_from(&headers);
    let update = AlertRuleUpdate {
        enabled: dto.enabled,
        threshold: dto.threshold,
        severity: dto.severity,
        cooldown_secs: dto.cooldown_secs,
    };
    let rule = state.alert_rules_uc.update(&id, update).await?;

    // AUDITE (point O5) : desactiver une regle, ou relever son seuil, revient a
    // AVEUGLER la supervision. C'est le genre de changement qu'on veut pouvoir
    // dater et attribuer apres un incident — « l'alerte n'a pas sonne » et
    // « quelqu'un l'a eteinte trois jours plus tot » ne se distinguent pas sans
    // cette ligne. `warn` quand la regle est desactivee, `info` sinon : le
    // journal doit faire ressortir la coupure, pas un ajustement de seuil.
    let severite = if rule.enabled { "info" } else { "warn" };
    crate::ops::handlers::security::record_event(
        &state.server_events,
        &actor,
        None,
        "alert_rule.update",
        Some(&rule.id),
        severite,
        serde_json::json!({
            "id": rule.id,
            "enabled": rule.enabled,
            "threshold": rule.threshold,
            "severity": rule.severity,
            "cooldown_secs": rule.cooldown_secs,
        }),
    )
    .await;

    Ok(Json(rule.into()))
}
