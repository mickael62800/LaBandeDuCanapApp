use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::idea::{Idea, IdeaDetail, IdeaMessage};
use crate::sentinel::domain::enums::community::idea_status::IdeaStatus;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_ideas::{
    AddIdeaMessageCommand, CreateIdeaCommand, DecideIdeaCommand, ManageIdeasUseCase,
};
use crate::sentinel::ports::outbound::community::idea_repository::{IdeaFilters, IdeaRepository};

/// Bornes de saisie (miroir des contraintes Discord : 100 pour un titre de
/// salon lisible, 2000 pour un champ de modale).
const TITLE_MAX: usize = 100;
const DESCRIPTION_MAX: usize = 2000;
const CATEGORY_MAX: usize = 50;
const REASON_MAX: usize = 1000;
const MESSAGE_MAX: usize = 4000;

pub struct ManageIdeasService {
    repo: Arc<dyn IdeaRepository>,
}

impl ManageIdeasService {
    pub fn new(repo: Arc<dyn IdeaRepository>) -> Self {
        Self { repo }
    }

    fn validate_create(cmd: &CreateIdeaCommand) -> Result<(), DomainError> {
        if cmd.guild_id.trim().is_empty() {
            return Err(DomainError::ValidationError("guild_id est requis".into()));
        }
        if cmd.title.trim().is_empty() {
            return Err(DomainError::ValidationError("Le titre est requis".into()));
        }
        if cmd.title.chars().count() > TITLE_MAX {
            return Err(DomainError::ValidationError(format!(
                "Titre trop long (max {TITLE_MAX})"
            )));
        }
        if cmd.description.chars().count() > DESCRIPTION_MAX {
            return Err(DomainError::ValidationError(format!(
                "Description trop longue (max {DESCRIPTION_MAX})"
            )));
        }
        if cmd.category.chars().count() > CATEGORY_MAX {
            return Err(DomainError::ValidationError(format!(
                "Categorie trop longue (max {CATEGORY_MAX})"
            )));
        }
        if cmd.author_id.trim().is_empty() {
            return Err(DomainError::ValidationError("author_id est requis".into()));
        }
        Ok(())
    }

    fn parse_status(s: &str) -> Result<IdeaStatus, DomainError> {
        IdeaStatus::from_str(s).ok_or_else(|| {
            DomainError::ValidationError(format!(
                "statut invalide: {s} (attendu {})",
                IdeaStatus::VALID_VALUES.join("/")
            ))
        })
    }
}

#[async_trait]
impl ManageIdeasUseCase for ManageIdeasService {
    async fn list(
        &self,
        filters: IdeaFilters<'_>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Idea>, DomainError> {
        // Bornes dures : une page web ne doit pas pouvoir vider la table.
        let limit = limit.clamp(1, 200);
        let offset = offset.max(0);
        if let Some(s) = filters.status {
            Self::parse_status(s)?;
        }
        self.repo.find_all(filters, limit, offset).await
    }

    async fn get(&self, id: Uuid) -> Result<Idea, DomainError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Idee {id} introuvable")))
    }

    async fn get_detail(&self, id: Uuid) -> Result<IdeaDetail, DomainError> {
        let idea = self.get(id).await?;
        let messages = self.repo.find_messages(id).await?;
        Ok(IdeaDetail { idea, messages })
    }

    async fn get_by_channel(&self, channel_id: &str) -> Result<Option<Idea>, DomainError> {
        self.repo.find_by_channel(channel_id).await
    }

    async fn create(&self, cmd: CreateIdeaCommand) -> Result<Idea, DomainError> {
        Self::validate_create(&cmd)?;
        let now = Utc::now();
        let idea = Idea {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            title: cmd.title.trim().to_string(),
            description: cmd.description.trim().to_string(),
            status: IdeaStatus::Nouvelle.as_str().to_string(),
            category: cmd.category.trim().to_string(),
            author_id: cmd.author_id,
            author_name: cmd.author_name,
            channel_id: cmd.channel_id,
            decided_by: None,
            decided_by_name: None,
            decision_reason: None,
            decided_at: None,
            created_at: now,
            updated_at: now,
        };
        self.repo.create(&idea).await?;
        Ok(idea)
    }

    async fn decide(&self, cmd: DecideIdeaCommand) -> Result<Idea, DomainError> {
        let target = Self::parse_status(&cmd.status)?;
        let mut idea = self.get(cmd.id).await?;
        let current = Self::parse_status(&idea.status)?;

        if !IdeaStatus::can_transition(current, target) {
            return Err(DomainError::ValidationError(format!(
                "Transition interdite : {} -> {}",
                current.label(),
                target.label()
            )));
        }
        if let Some(reason) = &cmd.reason {
            if reason.chars().count() > REASON_MAX {
                return Err(DomainError::ValidationError(format!(
                    "Motif trop long (max {REASON_MAX})"
                )));
            }
        }

        idea.status = target.as_str().to_string();
        idea.decided_by = Some(cmd.decided_by);
        idea.decided_by_name = Some(cmd.decided_by_name);
        // Un motif vide efface le precedent plutot que de le figer.
        idea.decision_reason = cmd.reason.filter(|r| !r.trim().is_empty());
        // `decided_at` ne date que les vraies decisions : passer "en discussion"
        // n'en est pas une, l'idee reste ouverte.
        idea.decided_at = if target.is_decided() {
            Some(Utc::now())
        } else {
            None
        };
        idea.updated_at = Utc::now();

        self.repo.update(&idea).await?;
        Ok(idea)
    }

    async fn set_channel(&self, id: Uuid, channel_id: Option<&str>) -> Result<Idea, DomainError> {
        let mut idea = self.get(id).await?;
        idea.channel_id = channel_id.map(|c| c.to_string());
        idea.updated_at = Utc::now();
        self.repo.update(&idea).await?;
        Ok(idea)
    }

    async fn add_message(&self, cmd: AddIdeaMessageCommand) -> Result<IdeaMessage, DomainError> {
        if cmd.content.trim().is_empty() {
            return Err(DomainError::ValidationError("Message vide".into()));
        }
        if cmd.content.chars().count() > MESSAGE_MAX {
            return Err(DomainError::ValidationError(format!(
                "Message trop long (max {MESSAGE_MAX})"
            )));
        }
        // Verifie l'existence de l'idee : evite les messages orphelins.
        self.get(cmd.idea_id).await?;

        let message = IdeaMessage {
            id: Uuid::new_v4(),
            idea_id: cmd.idea_id,
            author_name: cmd.author_name,
            author_role: cmd.author_role,
            content: cmd.content,
            created_at: Utc::now(),
        };
        self.repo.save_message(&message).await?;
        Ok(message)
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        self.repo.delete(id).await
    }

    async fn count_open_by_author(
        &self,
        guild_id: &str,
        author_id: &str,
    ) -> Result<i64, DomainError> {
        self.repo.count_open_by_author(guild_id, author_id).await
    }
}


#[cfg(test)]
#[path = "tests/manage_ideas_extended.rs"]
mod tests;
