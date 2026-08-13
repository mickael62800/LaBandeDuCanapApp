//! Lecture du snapshot de conteneurs publie par `ops-agent`.

use platform_core::ops::domain::entities::container_monitor::{
    ContainerMonitorState, REDIS_STATE_KEY,
};

pub async fn load(redis: &redis::aio::ConnectionManager) -> Result<ContainerMonitorState, String> {
    let mut connection = redis.clone();
    let encoded: Option<String> = redis::cmd("GET")
        .arg(REDIS_STATE_KEY)
        .query_async(&mut connection)
        .await
        .map_err(|error| error.to_string())?;
    encoded
        .map(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
        .transpose()
        .map(|state| state.unwrap_or_default())
}
