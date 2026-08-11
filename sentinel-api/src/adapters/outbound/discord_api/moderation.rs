use super::*;

impl DiscordApiService {
    pub(super) async fn ban_user_impl(
        &self,
        guild_id: &str,
        user_id: &str,
        reason: &str,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;

        let url = format!(
            "https://discord.com/api/v10/guilds/{}/bans/{}",
            guild_id, user_id
        );

        let resp = self
            .client
            .put(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({
                "delete_message_seconds": 86400,
                "reason": reason,
            }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord ban failed ({status}): {body}"
            )));
        }

        Ok(())
    }

    pub(super) async fn unban_user_impl(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;

        let url = format!(
            "https://discord.com/api/v10/guilds/{}/bans/{}",
            guild_id, user_id
        );

        let resp = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        let status = resp.status();
        if !status.is_success() && status.as_u16() != 404 {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord unban failed ({status}): {body}"
            )));
        }

        Ok(())
    }

    pub(super) async fn remove_timeout_impl(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;

        let url = format!(
            "https://discord.com/api/v10/guilds/{}/members/{}",
            guild_id, user_id
        );

        let resp = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({ "communication_disabled_until": null }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        let status = resp.status();
        if !status.is_success() && status.as_u16() != 404 {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord remove_timeout failed ({status}): {body}"
            )));
        }

        Ok(())
    }

    pub(super) async fn apply_timeout_impl(
        &self,
        guild_id: &str,
        user_id: &str,
        duration_seconds: u64,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;

        // Discord max : 28 jours (2 419 200 secondes).
        const MAX_TIMEOUT_SECS: u64 = 28 * 24 * 3600;
        let dur = duration_seconds.min(MAX_TIMEOUT_SECS);
        let until = chrono::Utc::now() + chrono::Duration::seconds(dur as i64);
        let until_str = until.to_rfc3339();

        let url = format!(
            "https://discord.com/api/v10/guilds/{}/members/{}",
            guild_id, user_id
        );

        let resp = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({ "communication_disabled_until": until_str }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord apply_timeout failed ({status}): {body}"
            )));
        }

        Ok(())
    }
}
