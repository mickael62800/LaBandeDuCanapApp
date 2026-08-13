use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use redis::AsyncCommands;
use tracing::warn;

use crate::sentinel::adapters::inbound::http::dto::audit::analytics::*;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::bootstrap::state::AuditState;

/// TTL du cache analytics (5 minutes).
const ANALYTICS_CACHE_TTL: u64 = 300;

/// Construit la cle de cache pour un endpoint analytics.
fn cache_key(endpoint: &str, guild_id: Option<&str>, days: i32, limit: Option<i64>) -> String {
    let gid = guild_id.unwrap_or("global");
    match limit {
        Some(l) => format!("analytics:{endpoint}:{gid}:{days}:{l}"),
        None => format!("analytics:{endpoint}:{gid}:{days}"),
    }
}

/// Tente de lire une valeur depuis le cache Redis.
async fn try_cache_get<T: serde::de::DeserializeOwned>(state: &AuditState, key: &str) -> Option<T> {
    let mut conn = state
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .ok()?;
    let json: Option<String> = conn.get(key).await.ok()?;
    let json = json?;
    serde_json::from_str(&json).ok()
}

/// Ecrit une valeur dans le cache Redis.
async fn try_cache_set<T: serde::Serialize>(state: &AuditState, key: &str, value: &T) {
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(value) {
            let result: Result<(), _> = conn.set_ex(key, json, ANALYTICS_CACHE_TTL).await;
            if let Err(e) = result {
                warn!(error = %e, key = key, "Erreur ecriture cache analytics");
            }
        }
    }
}

/// Wrapper cache-first generique : tente le cache, sinon execute `compute`,
/// ecrit le resultat en cache (TTL `ANALYTICS_CACHE_TTL`) puis renvoie le Json.
/// Factorise le pattern repete par les handlers analytics.
async fn cached<T, F, Fut>(state: &AuditState, key: &str, compute: F) -> Result<Json<T>, ApiError>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, ApiError>>,
{
    if let Some(hit) = try_cache_get::<T>(state, key).await {
        return Ok(Json(hit));
    }

    let value = compute().await?;
    try_cache_set(state, key, &value).await;

    Ok(Json(value))
}

/// GET /api/analytics — Retourne toutes les analytics en une seule requete (cache 5min).
pub async fn get_full_analytics(
    State(state): State<AuditState>,
    _user: Option<Extension<WebUser>>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<FullAnalyticsDto>, ApiError> {
    let days = params.days();
    let limit = params.limit();
    let guild_id = params.guild_id.as_deref();
    let key = cache_key("full", guild_id, days, Some(limit));

    cached(&state, &key, || async {
        let (heatmap, distribution, infractors, trend, peaks) = tokio::try_join!(
            state.analytics_repo.get_heatmap(guild_id, days),
            state.analytics_repo.get_action_distribution(guild_id, days),
            state
                .analytics_repo
                .get_top_infractors(guild_id, days, limit, 0),
            state.analytics_repo.get_moderation_trend(guild_id, days),
            state.analytics_repo.get_peak_hours(guild_id, days),
        )?;

        Ok(FullAnalyticsDto {
            heatmap: heatmap.into_iter().map(Into::into).collect(),
            action_distribution: distribution.into_iter().map(Into::into).collect(),
            top_infractors: infractors.into_iter().map(Into::into).collect(),
            moderation_trend: trend.into_iter().map(Into::into).collect(),
            peak_hours: peaks.into_iter().map(Into::into).collect(),
        })
    })
    .await
}

/// POST /api/analytics/reset?guild_id=X
///
/// Reset les compteurs d'activite (hourly_activity + daily_activity) pour
/// la guild specifiee. NE TOUCHE PAS aux infractions/audit_logs (donnees
/// d'audit reelles, conservees pour la chaine de moderation).
/// Flush egalement le cache Redis analytics:* pour la guild.
#[derive(serde::Serialize)]
pub struct ResetAnalyticsResponse {
    pub deleted_rows: u64,
}

pub async fn reset_analytics(
    State(state): State<AuditState>,
    _user: Option<Extension<WebUser>>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<ResetAnalyticsResponse>, ApiError> {
    let guild_id = params.guild_id.as_deref().ok_or_else(|| {
        ApiError::from(
            platform_core::sentinel::domain::errors::DomainError::ValidationError(
                "guild_id requis".into(),
            ),
        )
    })?;

    // Guild fourni en query -> check_role_for_guild (lookup explicite, bypass
    // superadmin, pass-through si pas de WebUser = appel interne).

    let deleted_rows = state.analytics_repo.reset_activity(guild_id).await?;

    // Flush du cache Redis (toutes les cles analytics:*:<guild_id>:*).
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        let pattern = format!("analytics:*:{guild_id}:*");
        if let Ok(keys) = conn.keys::<_, Vec<String>>(pattern).await {
            if !keys.is_empty() {
                let _: Result<(), _> = conn.del(keys).await;
            }
        }
    }

    Ok(Json(ResetAnalyticsResponse { deleted_rows }))
}

#[cfg(test)]
#[path = "tests/analytics.rs"]
mod tests;
