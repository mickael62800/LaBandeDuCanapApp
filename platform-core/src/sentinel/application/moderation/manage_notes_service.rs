use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::moderation::user_note::UserNote;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::manage_notes::AddNoteCommand;
use crate::sentinel::ports::inbound::moderation::manage_notes::ManageNotesUseCase;
use crate::sentinel::ports::outbound::moderation::notes_repository::NotesRepository;

pub struct ManageNotesService {
    repo: Arc<dyn NotesRepository>,
}

impl ManageNotesService {
    pub fn new(repo: Arc<dyn NotesRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageNotesUseCase for ManageNotesService {
    async fn add_note(&self, cmd: AddNoteCommand) -> Result<UserNote, DomainError> {
        let valid_categories = ["general", "warning", "positive", "context"];
        if !valid_categories.contains(&cmd.category.as_str()) {
            return Err(DomainError::ValidationError(format!(
                "Categorie invalide '{}'. Valeurs acceptees : general, warning, positive, context",
                cmd.category
            )));
        }

        let now = Utc::now();
        let note = UserNote {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            user_id: cmd.user_id,
            author_id: cmd.author_id,
            author_name: cmd.author_name,
            content: cmd.content,
            category: cmd.category,
            created_at: now,
            updated_at: now,
        };

        self.repo.save(&note).await?;
        Ok(note)
    }

    async fn get_notes(&self, guild_id: &str, user_id: &str) -> Result<Vec<UserNote>, DomainError> {
        self.repo.find_by_user(guild_id, user_id).await
    }

    async fn note_guild_id(&self, note_id: &str) -> Result<Option<String>, DomainError> {
        self.repo.find_guild_id(note_id).await
    }

    async fn delete_note(&self, note_id: &str) -> Result<(), DomainError> {
        self.repo.delete(note_id).await
    }
}
