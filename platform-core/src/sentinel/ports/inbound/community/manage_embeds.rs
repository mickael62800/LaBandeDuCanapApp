use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::embed::{Embed, EmbedField, RenderedEmbedPost};
use crate::sentinel::domain::errors::DomainError;

/// Champs modifiables d'un embed (create/update partagent la meme forme).
pub struct EmbedInput {
    pub name: String,
    pub content: String,
    pub author_name: String,
    pub author_icon_url: String,
    pub author_url: String,
    pub title: String,
    pub title_url: String,
    pub description: String,
    pub color: Option<i32>,
    pub image_url: String,
    pub thumbnail_url: String,
    pub footer_text: String,
    pub footer_icon_url: String,
    pub show_timestamp: bool,
    pub fields: Vec<EmbedField>,
}

#[async_trait]
pub trait ManageEmbedsUseCase: Send + Sync {
    async fn create(
        &self,
        guild_id: &str,
        created_by: &str,
        input: EmbedInput,
    ) -> Result<Embed, DomainError>;
    async fn update(&self, id: Uuid, input: EmbedInput) -> Result<Embed, DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    async fn get(&self, id: Uuid) -> Result<Embed, DomainError>;
    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Embed>, DomainError>;

    /// Prepare le payload pour POSTER l'embed dans `channel_id` (nouveau
    /// message). Le bot posera puis rapportera l'id via `record_posted`.
    async fn prepare_post(
        &self,
        id: Uuid,
        channel_id: &str,
    ) -> Result<RenderedEmbedPost, DomainError>;

    /// Prepare le payload pour EDITER le dernier message poste de cet embed.
    /// Erreur si l'embed n'a jamais ete poste.
    async fn prepare_edit(&self, id: Uuid) -> Result<RenderedEmbedPost, DomainError>;

    /// Appele par le bot apres un post reussi : memorise (channel, message).
    async fn record_posted(
        &self,
        id: Uuid,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), DomainError>;
}
