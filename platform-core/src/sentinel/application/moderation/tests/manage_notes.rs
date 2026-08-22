use super::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::sentinel::application::moderation::manage_notes_service::ManageNotesService;
use crate::sentinel::domain::entities::moderation::user_note::UserNote;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::manage_notes::{AddNoteCommand, ManageNotesUseCase};
use crate::sentinel::ports::outbound::moderation::notes_repository::NotesRepository;
use async_trait::async_trait;

#[derive(Default)]
struct MockNotesRepo {
    notes: Mutex<Vec<UserNote>>,
}

#[async_trait]
impl NotesRepository for MockNotesRepo {
    async fn save(&self, note: &UserNote) -> Result<(), DomainError> {
        self.notes.lock().await.push(note.clone());
        Ok(())
    }

    async fn find_by_user(&self, guild_id: &str, user_id: &str) -> Result<Vec<UserNote>, DomainError> {
        Ok(self.notes.lock().await.iter()
            .filter(|n| n.guild_id == guild_id && n.user_id == user_id)
            .cloned().collect())
    }

    async fn delete(&self, note_id: &str) -> Result<(), DomainError> {
        let uuid = Uuid::parse_str(note_id).map_err(|_| DomainError::NotFound("note".into()))?;
        let mut notes = self.notes.lock().await;
        notes.retain(|n| n.id != uuid);
        Ok(())
    }

    async fn find_guild_id(&self, note_id: &str) -> Result<Option<String>, DomainError> {
        let uuid = Uuid::parse_str(note_id).ok();
        Ok(self.notes.lock().await.iter()
            .find(|n| Some(n.id) == uuid)
            .map(|n| n.guild_id.clone()))
    }
}

fn build_service() -> ManageNotesService {
    ManageNotesService::new(Arc::new(MockNotesRepo::default()) as Arc<dyn NotesRepository>)
}

#[tokio::test]
async fn add_note_valid() {
    let svc = build_service();
    let cmd = AddNoteCommand {
        guild_id: "g1".into(),
        user_id: "u1".into(),
        author_id: "mod".into(),
        author_name: "Moderator".into(),
        content: "Test note".into(),
        category: "general".into(),
    };
    let result = svc.add_note(cmd).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn add_note_empty_content() {
    let svc = build_service();
    let cmd = AddNoteCommand {
        guild_id: "g1".into(),
        user_id: "u1".into(),
        author_id: "mod".into(),
        author_name: "Moderator".into(),
        content: "".into(),
        category: "general".into(),
    };
    let result = svc.add_note(cmd).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn get_notes_empty() {
    let svc = build_service();
    let result = svc.get_notes("g1", "u1").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
async fn delete_note_invalid_id() {
    let svc = build_service();
    let result = svc.delete_note("invalid-uuid").await;
    assert!(result.is_err());
}
