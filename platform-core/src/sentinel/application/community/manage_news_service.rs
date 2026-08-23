//! Annonces du site.
//!
//! Le service normalise les champs libres et verrouille le chemin d'image :
//! seul un chemin relatif servi par le site est accepte.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::news::{NewsPost, UpsertNewsCommand};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_news::ManageNewsUseCase;
use crate::sentinel::ports::outbound::community::news_repository::NewsRepository;

const MAX_TITLE_CHARS: usize = 160;
const MAX_BODY_CHARS: usize = 4000;
const MAX_LIMIT: i64 = 50;

pub struct ManageNewsService {
    repo: Arc<dyn NewsRepository>,
}

impl ManageNewsService {
    pub fn new(repo: Arc<dyn NewsRepository>) -> Self {
        Self { repo }
    }

    fn sanitize(mut cmd: UpsertNewsCommand) -> Result<UpsertNewsCommand, DomainError> {
        cmd.title = cmd.title.trim().to_string();
        if cmd.title.is_empty() {
            return Err(DomainError::ValidationError("titre obligatoire".into()));
        }
        if cmd.title.chars().count() > MAX_TITLE_CHARS {
            return Err(DomainError::ValidationError(
                "titre limite a 160 caracteres".into(),
            ));
        }

        cmd.body = cmd.body.trim().chars().take(MAX_BODY_CHARS).collect();
        if cmd.body.is_empty() {
            return Err(DomainError::ValidationError("texte obligatoire".into()));
        }

        // Chemin relatif uniquement. Accepter une URL absolue ouvrirait la
        // porte a un `javascript:` dans un attribut `src`, et a un domaine
        // fige en base — le meme choix que pour les jaquettes de jeu.
        cmd.image_url = cmd
            .image_url
            .map(|u| u.trim().to_string())
            .filter(|u| !u.is_empty())
            .map(|u| {
                if u.starts_with('/') && !u.starts_with("//") {
                    Ok(u)
                } else {
                    Err(DomainError::ValidationError(
                        "l'image doit etre un chemin relatif, par exemple /imgs/annonce.jpg".into(),
                    ))
                }
            })
            .transpose()?;

        Ok(cmd)
    }
}

#[async_trait]
impl ManageNewsUseCase for ManageNewsService {
    async fn list(
        &self,
        guild_id: &str,
        published_only: bool,
        limit: i64,
    ) -> Result<Vec<NewsPost>, DomainError> {
        self.repo
            .list(guild_id, published_only, limit.clamp(1, MAX_LIMIT))
            .await
    }

    async fn get(&self, id: Uuid) -> Result<NewsPost, DomainError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("annonce introuvable".into()))
    }

    async fn create(&self, cmd: UpsertNewsCommand) -> Result<NewsPost, DomainError> {
        self.repo.create(&Self::sanitize(cmd)?).await
    }

    async fn update(&self, id: Uuid, cmd: UpsertNewsCommand) -> Result<NewsPost, DomainError> {
        self.repo
            .update(id, &Self::sanitize(cmd)?)
            .await?
            .ok_or_else(|| DomainError::NotFound("annonce introuvable".into()))
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        if self.repo.delete(id).await? {
            Ok(())
        } else {
            Err(DomainError::NotFound("annonce introuvable".into()))
        }
    }
}
