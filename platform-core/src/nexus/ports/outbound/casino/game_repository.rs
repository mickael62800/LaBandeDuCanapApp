use async_trait::async_trait;

use crate::nexus::domain::entities::system::discord_ids::ChannelId;
use crate::nexus::domain::entities::system::discord_ids::GuildId;
use crate::nexus::domain::entities::system::discord_ids::MessageId;
use crate::nexus::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct Game {
    pub id: String,
    pub guild_id: GuildId,
    pub game_name: String,
    pub created_by: String,
    pub created_at: String,
    pub emoji: Option<String>,
    pub category: Option<String>,
    pub role_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GamePanel {
    pub id: String,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub category: Option<String>,
}

#[async_trait]
pub trait GameRepository: Send + Sync {
    async fn list(&self, guild_id: &str) -> Result<Vec<Game>, DomainError>;
    async fn list_by_category(
        &self,
        guild_id: &str,
        category: Option<&str>,
    ) -> Result<Vec<Game>, DomainError>;
    async fn create(
        &self,
        guild_id: &str,
        game_name: &str,
        created_by: &str,
        emoji: Option<&str>,
        category: Option<&str>,
        role_id: Option<&str>,
    ) -> Result<Game, DomainError>;
    async fn update(
        &self,
        guild_id: &str,
        game_id: &str,
        game_name: Option<&str>,
        emoji: Option<Option<&str>>,
        category: Option<Option<&str>>,
    ) -> Result<Option<Game>, DomainError>;
    async fn delete(&self, guild_id: &str, game_id: &str) -> Result<bool, DomainError>;
    async fn find_by_name(
        &self,
        guild_id: &str,
        game_name: &str,
    ) -> Result<Option<Game>, DomainError>;
    async fn set_role_id(
        &self,
        guild_id: &str,
        game_id: &str,
        role_id: Option<&str>,
    ) -> Result<Option<Game>, DomainError>;

    // Panels
    async fn save_panel(
        &self,
        guild_id: &str,
        channel_id: &str,
        message_id: &str,
        category: Option<&str>,
    ) -> Result<GamePanel, DomainError>;
    async fn find_panel_by_message(
        &self,
        guild_id: &str,
        message_id: &str,
    ) -> Result<Option<GamePanel>, DomainError>;
    async fn list_panels(&self, guild_id: &str) -> Result<Vec<GamePanel>, DomainError>;
    /// Oublie un panneau dont le message n'existe plus dans Discord. Sans cela,
    /// la reconciliation signalerait indefiniment le meme ecart.
    /// Retourne `false` si aucun panneau ne portait ce message.
    async fn delete_panel(&self, guild_id: &str, message_id: &str) -> Result<bool, DomainError>;
}
