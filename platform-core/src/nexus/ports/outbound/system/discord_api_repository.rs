use crate::nexus::domain::errors::DomainError;
use async_trait::async_trait;

#[async_trait]
pub trait DiscordApiRepository: Send + Sync {
    /// Upload un emoji custom sur un serveur Discord.
    /// Retourne un tuple `(emoji_id, emoji_name)`.
    async fn upload_emoji(
        &self,
        guild_id: &str,
        name: &str,
        image_bytes: &[u8],
        mime: &str,
    ) -> Result<(String, String), DomainError>;
}
