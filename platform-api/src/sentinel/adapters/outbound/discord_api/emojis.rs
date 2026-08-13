use super::*;

impl DiscordApiService {
    pub(super) async fn list_emojis_impl(
        &self,
        guild_id: &str,
    ) -> Result<Vec<DiscordEmoji>, DomainError> {
        ensure_snowflake(guild_id)?;
        self.ensure_configured()?;
        let url = format!("https://discord.com/api/v10/guilds/{guild_id}/emojis");

        let res = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await;
        let res = match res {
            Ok(r) => r,
            Err(e) => {
                return Err(DomainError::Internal(format!(
                    "HTTP error list_emojis: {e}"
                )))
            }
        };

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord API list_emojis {status}: {body}"
            )));
        }

        let emojis: Vec<DiscordEmoji> = res
            .json()
            .await
            .map_err(|e| DomainError::Internal(format!("Parse error list_emojis: {e}")))?;
        Ok(emojis)
    }

    pub(super) async fn upload_emoji_impl(
        &self,
        guild_id: &str,
        name: &str,
        image_bytes: &[u8],
        mime: &str,
    ) -> Result<(String, String, bool), DomainError> {
        self.ensure_configured()?;

        if image_bytes.len() > 256 * 1024 {
            return Err(DomainError::ValidationError(
                "L'image depasse 256 KB apres encodage.".into(),
            ));
        }

        let b64 = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(image_bytes)
        };
        let data_uri = format!("data:{};base64,{}", mime, b64);

        let url = format!("https://discord.com/api/v10/guilds/{}/emojis", guild_id);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({
                "name": name,
                "image": data_uri,
            }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                403 => DomainError::Forbidden(format!(
                    "Le bot n'a pas la permission de gerer les emojis sur ce serveur. {body}"
                )),
                429 => DomainError::RateLimited(format!(
                    "Trop de requetes vers Discord, reessayez dans quelques instants. {body}"
                )),
                400 if body.contains("Maximum number of emojis") => DomainError::ValidationError(
                    "Le serveur d'hebergement est plein (quota d'emojis atteint).".into(),
                ),
                400 => DomainError::ValidationError(format!("Image ou nom invalide : {body}")),
                404 => DomainError::NotFound(
                    "Serveur Discord introuvable (le bot y est-il present ?)".into(),
                ),
                _ => DomainError::Internal(format!("Discord upload emoji ({status}): {body}")),
            });
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord upload emoji parse: {e}")))?;
        let id = body
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DomainError::Internal("Discord n'a pas renvoye l'id de l'emoji".into()))?
            .to_string();
        let returned_name = body
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(name)
            .to_string();
        let animated = body
            .get("animated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        Ok((id, returned_name, animated))
    }
}
