use async_trait::async_trait;
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::system::discord_api_repository::DiscordApiRepository;
use reqwest::Client;

pub struct ReqwestDiscordApiClient {
    client: Client,
    token: String,
}

impl ReqwestDiscordApiClient {
    pub fn new(token: String) -> Self {
        Self {
            client: Client::new(),
            token,
        }
    }
}

#[async_trait]
impl DiscordApiRepository for ReqwestDiscordApiClient {
    async fn upload_emoji(
        &self,
        guild_id: &str,
        name: &str,
        image_bytes: &[u8],
        mime: &str,
    ) -> Result<(String, String), DomainError> {
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

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord upload_emoji failed ({status}): {body}"
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DomainError::Internal(format!("Invalid Discord response: {e}")))?;

        let id = json["id"].as_str().unwrap_or_default().to_string();
        let name = json["name"].as_str().unwrap_or_default().to_string();

        Ok((id, name))
    }
}
