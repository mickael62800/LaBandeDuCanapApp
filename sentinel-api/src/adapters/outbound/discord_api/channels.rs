use super::*;

impl DiscordApiService {
    pub(super) async fn list_text_channels_impl(
        &self,
        guild_id: &str,
    ) -> Result<Vec<DiscordChannel>, DomainError> {
        self.ensure_configured()?;

        let url = format!("https://discord.com/api/v10/guilds/{}/channels", guild_id);
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
                "Discord list channels failed ({status}): {body}"
            )));
        }

        parse_channels(resp, |ty| match ty {
            0 => Some("text"),
            5 => Some("announcement"),
            _ => None,
        })
        .await
    }

    pub(super) async fn list_all_channels_impl(
        &self,
        guild_id: &str,
    ) -> Result<Vec<DiscordChannel>, DomainError> {
        self.ensure_configured()?;

        let url = format!("https://discord.com/api/v10/guilds/{}/channels", guild_id);
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
                "Discord list channels failed ({status}): {body}"
            )));
        }

        parse_channels(resp, |ty| match ty {
            0 => Some("text"),
            5 => Some("announcement"),
            2 => Some("voice"),
            4 => Some("category"),
            13 => Some("stage"),
            // Les forums sont creables depuis le constructeur de salons : les
            // omettre ici les rendait invisibles apres creation, donc
            // impossibles a supprimer depuis le panel et faciles a dupliquer.
            15 => Some("forum"),
            _ => None,
        })
        .await
    }

    pub(super) async fn create_channel_impl(
        &self,
        guild_id: &str,
        spec: &NewChannel<'_>,
    ) -> Result<String, DomainError> {
        self.ensure_configured()?;
        let url = format!("https://discord.com/api/v10/guilds/{}/channels", guild_id);

        let mut body = serde_json::json!({
            "name": spec.name,
            "type": spec.kind,
            "nsfw": spec.nsfw,
        });
        if let Some(parent) = spec.parent_id {
            body["parent_id"] = serde_json::Value::String(parent.to_string());
        }
        if let Some(topic) = spec.topic.filter(|t| !t.is_empty()) {
            body["topic"] = serde_json::Value::String(topic.to_string());
        }
        if spec.slowmode > 0 {
            body["rate_limit_per_user"] = serde_json::json!(spec.slowmode);
        }
        if let Some(limit) = spec.user_limit {
            body["user_limit"] = serde_json::json!(limit);
        }
        if !spec.overwrites.is_empty() {
            // Les bitfields Discord se transportent en CHAINES : ils depassent
            // la precision entiere de JSON.
            body["permission_overwrites"] = serde_json::Value::Array(
                spec.overwrites
                    .iter()
                    .map(|ow| {
                        serde_json::json!({
                            "id": ow.role_id,
                            "type": 0, // 0 = role
                            "allow": ow.allow.to_string(),
                            "deny": ow.deny.to_string(),
                        })
                    })
                    .collect(),
            );
        }

        let resp = send_with_rate_limit(|| {
            self.client
                .post(&url)
                .header("Authorization", format!("Bot {}", self.token))
                .json(&body)
        })
        .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            // Ces messages atterrissent tels quels dans le compte rendu du
            // panel, en face du salon concerne : ils doivent dire quoi faire,
            // pas recracher la reponse de Discord.
            return Err(match status.as_u16() {
                403 => DomainError::Forbidden(
                    "Le bot n'a pas la permission « Gerer les salons » sur ce serveur, ou ne peut \
                     pas accorder une permission qu'il ne possede pas lui-meme."
                        .into(),
                ),
                429 => DomainError::RateLimited(
                    "Discord limite le rythme de creation. Les salons restants n'ont pas ete \
                     crees : relancez le plan dans quelques instants."
                        .into(),
                ),
                400 if body.contains("Maximum number of channels") => DomainError::ValidationError(
                    "Le serveur a atteint la limite Discord de 500 salons.".into(),
                ),
                400 => DomainError::ValidationError(format!("Salon refuse par Discord : {body}")),
                404 => DomainError::NotFound(
                    "Serveur Discord introuvable (le bot y est-il present ?)".into(),
                ),
                _ => DomainError::Internal(format!("Discord create_channel ({status}): {body}")),
            });
        }

        let created: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DomainError::Internal(format!("Parse error: {e}")))?;
        created
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| DomainError::Internal("Discord create_channel: reponse sans id".into()))
    }

    pub(super) async fn delete_channel_impl(&self, channel_id: &str) -> Result<(), DomainError> {
        self.ensure_configured()?;
        let url = format!("https://discord.com/api/v10/channels/{}", channel_id);

        let resp = send_with_rate_limit(|| {
            self.client
                .delete(&url)
                .header("Authorization", format!("Bot {}", self.token))
        })
        .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord delete_channel failed ({status}): {body}"
            )));
        }
        Ok(())
    }
}
