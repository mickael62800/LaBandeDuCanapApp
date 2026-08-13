use super::*;

impl DiscordApiService {
    pub(super) async fn list_roles_impl(
        &self,
        guild_id: &str,
    ) -> Result<Vec<DiscordRoleInfo>, DomainError> {
        self.ensure_configured()?;
        let url = format!("https://discord.com/api/v10/guilds/{}/roles", guild_id);

        let resp = send_with_rate_limit(|| {
            self.client
                .get(&url)
                .header("Authorization", format!("Bot {}", self.token))
        })
        .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord list_roles failed ({status}): {body}"
            )));
        }

        let raw: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord list_roles parse: {e}")))?;
        let mut roles: Vec<DiscordRoleInfo> = raw
            .into_iter()
            .filter_map(|r| {
                Some(DiscordRoleInfo {
                    id: r.get("id")?.as_str()?.to_string(),
                    name: r.get("name")?.as_str()?.to_string(),
                    color: r.get("color").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    position: r.get("position").and_then(|v| v.as_i64()).unwrap_or(0),
                    managed: r.get("managed").and_then(|v| v.as_bool()).unwrap_or(false),
                })
            })
            .collect();
        roles.sort_by_key(|role| std::cmp::Reverse(role.position));
        Ok(roles)
    }

    pub(super) async fn create_role_impl(
        &self,
        guild_id: &str,
        name: &str,
        color: u32,
        permissions: Option<&str>,
    ) -> Result<serde_json::Value, DomainError> {
        self.ensure_configured()?;
        let url = format!("https://discord.com/api/v10/guilds/{}/roles", guild_id);

        let mut body = serde_json::json!({
            "name": name,
            "color": color,
            "mentionable": false,
        });
        if let Some(perms) = permissions {
            body["permissions"] = serde_json::Value::String(perms.to_string());
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord create_role failed: {body}"
            )));
        }

        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| DomainError::Internal(format!("Parse error: {e}")))
    }

    pub(super) async fn edit_role_impl(
        &self,
        guild_id: &str,
        role_id: &str,
        name: Option<&str>,
        color: Option<u32>,
        permissions: Option<&str>,
        mentionable: Option<bool>,
        hoist: Option<bool>,
    ) -> Result<serde_json::Value, DomainError> {
        self.ensure_configured()?;
        let url = format!(
            "https://discord.com/api/v10/guilds/{}/roles/{}",
            guild_id, role_id
        );

        let mut body = serde_json::json!({});
        if let Some(n) = name {
            body["name"] = serde_json::Value::String(n.to_string());
        }
        if let Some(c) = color {
            body["color"] = serde_json::json!(c);
        }
        if let Some(p) = permissions {
            body["permissions"] = serde_json::Value::String(p.to_string());
        }
        if let Some(m) = mentionable {
            body["mentionable"] = serde_json::json!(m);
        }
        if let Some(h) = hoist {
            body["hoist"] = serde_json::json!(h);
        }

        let resp = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord edit_role failed: {body}"
            )));
        }

        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| DomainError::Internal(format!("Parse error: {e}")))
    }

    pub(super) async fn delete_role_impl(
        &self,
        guild_id: &str,
        role_id: &str,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;
        let url = format!(
            "https://discord.com/api/v10/guilds/{}/roles/{}",
            guild_id, role_id
        );

        let resp = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord delete_role failed: {body}"
            )));
        }

        Ok(())
    }
}
