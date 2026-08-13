use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::outbound::system::discord_api_repository::DiscordApiRepository;
use std::sync::Arc;

pub struct UploadEmojiUseCase {
    pub discord_api: Arc<dyn DiscordApiRepository>,
}

impl UploadEmojiUseCase {
    pub fn new(discord_api: Arc<dyn DiscordApiRepository>) -> Self {
        Self { discord_api }
    }

    /// Valide l'image et demande son upload a l'API Discord.
    /// Retourne (emoji_id, emoji_name).
    pub async fn execute(
        &self,
        guild_id: &str,
        name: &str,
        image_bytes: &[u8],
        mime: &str,
    ) -> Result<(String, String), DomainError> {
        // Validation basique de la taille (Discord limite a 256 KB)
        if image_bytes.len() > 256 * 1024 {
            return Err(DomainError::ValidationError(
                "L'image depasse 256 KB (limite Discord).".into(),
            ));
        }

        self.discord_api
            .upload_emoji(guild_id, name, image_bytes, mime)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockDiscordApi;

    #[async_trait]
    impl DiscordApiRepository for MockDiscordApi {
        async fn upload_emoji(
            &self,
            _guild_id: &str,
            name: &str,
            _image_bytes: &[u8],
            _mime: &str,
        ) -> Result<(String, String), DomainError> {
            Ok(("emoji_123".to_string(), name.to_string()))
        }
    }

    #[tokio::test]
    async fn test_upload_emoji_too_large() {
        let uc = UploadEmojiUseCase::new(Arc::new(MockDiscordApi));
        let large_bytes = vec![0u8; 256 * 1024 + 1];

        let res = uc
            .execute("guild1", "custom_emoji", &large_bytes, "image/png")
            .await;
        assert!(res.is_err());
        if let Err(DomainError::ValidationError(msg)) = res {
            assert!(msg.contains("256 KB"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[tokio::test]
    async fn test_upload_emoji_success() {
        let uc = UploadEmojiUseCase::new(Arc::new(MockDiscordApi));
        let valid_bytes = vec![0u8; 100];

        let (id, name) = uc
            .execute("guild1", "pepe_smirk", &valid_bytes, "image/png")
            .await
            .unwrap();
        assert_eq!(id, "emoji_123");
        assert_eq!(name, "pepe_smirk");
    }
}
