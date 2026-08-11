use super::*;

impl DiscordApiService {
    pub(super) async fn list_members_impl(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<DiscordMember>, DomainError> {
        self.ensure_configured()?;

        let mut all_members = Vec::new();
        let mut after: Option<String> = None;
        let page_size = std::cmp::min(limit, 1000);

        loop {
            let mut url = format!(
                "https://discord.com/api/v10/guilds/{}/members?limit={}",
                guild_id, page_size
            );
            if let Some(ref after_id) = after {
                url.push_str(&format!("&after={}", after_id));
            }

            let resp = self
                .client
                .get(&url)
                .header("Authorization", format!("Bot {}", self.token))
                .send()
                .await
                .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(DomainError::Internal(format!(
                    "Discord list members failed ({status}): {body}"
                )));
            }

            let members: Vec<serde_json::Value> = resp
                .json()
                .await
                .map_err(|e| DomainError::Internal(format!("Discord parse error: {e}")))?;

            if members.is_empty() {
                break;
            }

            for m in &members {
                let user = match m.get("user") {
                    Some(u) => u,
                    None => continue,
                };

                let id = user
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let username = user
                    .get("username")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let display_name = m
                    .get("nick")
                    .and_then(|v| v.as_str())
                    .or_else(|| user.get("global_name").and_then(|v| v.as_str()))
                    .map(|s| s.to_string());

                let avatar_hash = user.get("avatar").and_then(|v| v.as_str());
                let avatar_url = discord_avatar_url(&id, avatar_hash);

                if !id.is_empty() {
                    all_members.push(DiscordMember {
                        id,
                        username,
                        display_name,
                        avatar_url,
                    });
                }
            }

            if all_members.len() >= limit as usize || members.len() < page_size as usize {
                break;
            }

            after = all_members.last().map(|m| m.id.clone());
        }

        Ok(all_members)
    }

    pub(super) async fn send_dm_impl(
        &self,
        user_id: &str,
        content: &str,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;

        // 1. Creer un canal DM
        let dm_resp = self
            .client
            .post("https://discord.com/api/v10/users/@me/channels")
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({ "recipient_id": user_id }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord DM channel error: {e}")))?;

        if !dm_resp.status().is_success() {
            let body = dm_resp.text().await.unwrap_or_default();
            tracing::warn!("Impossible d'ouvrir un DM avec {user_id}: {body}");
            return Ok(()); // Ne pas faire echouer la suppression si le DM echoue
        }

        let channel: serde_json::Value = dm_resp
            .json()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord DM parse error: {e}")))?;
        let channel_id = channel["id"].as_str().unwrap_or_default();

        // 2. Envoyer le message
        let msg_resp = self
            .client
            .post(format!(
                "https://discord.com/api/v10/channels/{channel_id}/messages"
            ))
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({ "content": content }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord send DM error: {e}")))?;

        if !msg_resp.status().is_success() {
            let body = msg_resp.text().await.unwrap_or_default();
            tracing::warn!("Echec envoi DM a {user_id}: {body}");
        }

        Ok(())
    }
}
