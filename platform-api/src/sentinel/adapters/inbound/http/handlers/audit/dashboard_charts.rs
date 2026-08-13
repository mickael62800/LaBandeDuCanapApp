use axum::extract::Query;
use axum::extract::State;
use axum::Json;

use crate::sentinel::adapters::inbound::http::dto::audit::dashboard_charts::ChartQueryParams;
use crate::sentinel::adapters::inbound::http::dto::audit::dashboard_charts::DailyActivityDto;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::helpers::map_to_dtos;
use crate::sentinel::adapters::inbound::http::helpers::normalize_days;
use crate::sentinel::bootstrap::state::CommunityState;

pub async fn get_activity_trend(
    State(state): State<CommunityState>,
    Query(params): Query<ChartQueryParams>,
) -> Result<Json<Vec<DailyActivityDto>>, ApiError> {
    let days = normalize_days(params.days, 30, 90);
    let activity = state
        .daily_activity_repo
        .get_activity(params.guild_id.as_deref(), days)
        .await?;
    Ok(map_to_dtos(activity))
}
