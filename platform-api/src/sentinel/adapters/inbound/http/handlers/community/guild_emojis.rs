use axum::extract::State;
use axum::Json;
use redis::AsyncCommands;
use tracing::warn;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use crate::sentinel::bootstrap::state::CommunityState;
use platform_core::sentinel::ports::outbound::discord_api::DiscordEmoji;

const EMOJIS_CACHE_TTL_SECS: u64 = 600;

/// GET /api/guilds/{guild_id}/emojis — liste les emojis custom.
pub async fn list_emojis(
    State(state): State<CommunityState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<DiscordEmoji>>, ApiError> {
    let cache_key = format!("guild:emojis:{guild_id}");

    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(Some(json)) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Ok(emojis) = serde_json::from_str::<Vec<DiscordEmoji>>(&json) {
                return Ok(Json(emojis));
            }
        }
    }

    let emojis = state.discord_api.list_emojis(&guild_id).await?;

    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&emojis) {
            if let Err(e) = conn
                .set_ex::<_, _, ()>(&cache_key, json, EMOJIS_CACHE_TTL_SECS)
                .await
            {
                warn!(error = %e, cache_key = %cache_key, "Echec cache set emojis");
            }
        }
    }

    Ok(Json(emojis))
}
