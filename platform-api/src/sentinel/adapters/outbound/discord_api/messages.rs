use super::*;

impl DiscordApiService {
    pub(super) async fn send_channel_embed_impl(
        &self,
        channel_id: &str,
        embed: serde_json::Value,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;
        ensure_snowflake(channel_id)?;

        let resp = self
            .client
            .post(format!(
                "https://discord.com/api/v10/channels/{channel_id}/messages"
            ))
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({ "embeds": [embed] }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord send embed error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord a refuse l'embed (HTTP {status}) : {body}"
            )));
        }
        Ok(())
    }
}
