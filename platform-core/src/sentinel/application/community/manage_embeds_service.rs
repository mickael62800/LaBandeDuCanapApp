use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::embed::{Embed, RenderedEmbedPost};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_embeds::{EmbedInput, ManageEmbedsUseCase};
use crate::sentinel::ports::outbound::community::embed_repository::EmbedRepository;

pub struct ManageEmbedsService {
    repo: Arc<dyn EmbedRepository>,
}

impl ManageEmbedsService {
    pub fn new(repo: Arc<dyn EmbedRepository>) -> Self {
        Self { repo }
    }

    fn validate(input: &EmbedInput) -> Result<(), DomainError> {
        if input.name.trim().is_empty() {
            return Err(DomainError::ValidationError("Le nom est requis".into()));
        }
        if input.name.chars().count() > 100 {
            return Err(DomainError::ValidationError(
                "Nom trop long (max 100)".into(),
            ));
        }
        if input.title.chars().count() > 256 {
            return Err(DomainError::ValidationError(
                "Titre trop long (max 256)".into(),
            ));
        }
        if input.description.chars().count() > 4000 {
            return Err(DomainError::ValidationError(
                "Description trop longue (max 4000)".into(),
            ));
        }
        if input.fields.len() > 25 {
            return Err(DomainError::ValidationError("Max 25 champs".into()));
        }
        for f in &input.fields {
            if f.name.chars().count() > 256 || f.value.chars().count() > 1024 {
                return Err(DomainError::ValidationError(
                    "Champ trop long (name 256 / value 1024)".into(),
                ));
            }
        }
        Ok(())
    }

    fn apply(e: &mut Embed, input: EmbedInput) {
        e.name = input.name;
        e.content = input.content;
        e.author_name = input.author_name;
        e.author_icon_url = input.author_icon_url;
        e.author_url = input.author_url;
        e.title = input.title;
        e.title_url = input.title_url;
        e.description = input.description;
        e.color = input.color;
        e.image_url = input.image_url;
        e.thumbnail_url = input.thumbnail_url;
        e.footer_text = input.footer_text;
        e.footer_icon_url = input.footer_icon_url;
        e.show_timestamp = input.show_timestamp;
        e.fields = input.fields;
        e.updated_at = Utc::now();
    }
}

#[async_trait]
impl ManageEmbedsUseCase for ManageEmbedsService {
    async fn create(
        &self,
        guild_id: &str,
        created_by: &str,
        input: EmbedInput,
    ) -> Result<Embed, DomainError> {
        Self::validate(&input)?;
        let now = Utc::now();
        let mut e = Embed {
            id: Uuid::new_v4(),
            guild_id: guild_id.to_string(),
            name: String::new(),
            content: String::new(),
            author_name: String::new(),
            author_icon_url: String::new(),
            author_url: String::new(),
            title: String::new(),
            title_url: String::new(),
            description: String::new(),
            color: None,
            image_url: String::new(),
            thumbnail_url: String::new(),
            footer_text: String::new(),
            footer_icon_url: String::new(),
            show_timestamp: false,
            fields: Vec::new(),
            last_channel_id: None,
            last_message_id: None,
            created_by: created_by.to_string(),
            created_at: now,
            updated_at: now,
        };
        Self::apply(&mut e, input);
        self.repo.create(&e).await?;
        Ok(e)
    }

    async fn update(&self, id: Uuid, input: EmbedInput) -> Result<Embed, DomainError> {
        Self::validate(&input)?;
        let mut e = self.get(id).await?;
        Self::apply(&mut e, input);
        self.repo.update(&e).await?;
        Ok(e)
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        self.repo.delete(id).await
    }

    async fn get(&self, id: Uuid) -> Result<Embed, DomainError> {
        self.repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Embed {id} introuvable")))
    }

    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Embed>, DomainError> {
        self.repo.list_by_guild(guild_id).await
    }

    async fn prepare_post(
        &self,
        id: Uuid,
        channel_id: &str,
    ) -> Result<RenderedEmbedPost, DomainError> {
        let e = self.get(id).await?;
        if !e.has_visible_content() {
            return Err(DomainError::ValidationError(
                "L'embed est vide : ajoute au moins un titre, une description, une image ou un champ".into(),
            ));
        }
        Ok(RenderedEmbedPost::from_embed(
            &e,
            channel_id.to_string(),
            None,
        ))
    }

    async fn prepare_edit(&self, id: Uuid) -> Result<RenderedEmbedPost, DomainError> {
        let e = self.get(id).await?;
        let (ch, msg) = match (&e.last_channel_id, &e.last_message_id) {
            (Some(c), Some(m)) => (c.clone(), m.clone()),
            _ => {
                return Err(DomainError::ValidationError(
                    "Cet embed n'a jamais ete poste : poste-le d'abord".into(),
                ))
            }
        };
        Ok(RenderedEmbedPost::from_embed(&e, ch, Some(msg)))
    }

    async fn record_posted(
        &self,
        id: Uuid,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), DomainError> {
        self.repo.set_last_post(id, channel_id, message_id).await
    }
}
