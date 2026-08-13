const STREAM_KEY: &str = "sentinel:events";
const STREAM_MAXLEN: usize = 10_000;

pub async fn is_enabled(pool: &sqlx::PgPool, guild_id: &str, worker_name: &str) -> bool {
    let value: Option<String> = sqlx::query_scalar(
        "SELECT config_value FROM bot_guild_config \
         WHERE guild_id = $1 AND bot_name = $2 AND config_key = 'enabled'",
    )
    .bind(guild_id)
    .bind(worker_name)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    platform_common::config_flags::parse_enabled_flag(value.as_deref())
}

pub async fn publish_event<C>(conn: &mut C, payload: &str) -> redis::RedisResult<String>
where
    C: redis::aio::ConnectionLike + Send + Unpin,
{
    redis::cmd("XADD")
        .arg(STREAM_KEY)
        .arg("MAXLEN")
        .arg("~")
        .arg(STREAM_MAXLEN)
        .arg("*")
        .arg("payload")
        .arg(payload)
        .query_async(conn)
        .await
}

pub async fn publish_event_json<C>(conn: &mut C, payload: &serde_json::Value) -> Result<(), String>
where
    C: redis::aio::ConnectionLike + Send + Unpin,
{
    let serialized =
        serde_json::to_string(payload).map_err(|error| format!("serialize: {error}"))?;
    publish_event(conn, &serialized)
        .await
        .map_err(|error| format!("XADD: {error}"))?;
    Ok(())
}
