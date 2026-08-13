//! Tests unitaires du ManageNotesService.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::sentinel::application::moderation::manage_notes_service::ManageNotesService;
use crate::sentinel::domain::entities::moderation::user_note::*;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::manage_notes::*;
use crate::sentinel::ports::outbound::moderation::notes_repository::NotesRepository;

// ══════════════════════════════════════════════════════════
// In-memory Notes Repository
// ══════════════════════════════════════════════════════════

struct InMemoryNotesRepo {
    notes: Mutex<Vec<UserNote>>,
}

impl InMemoryNotesRepo {
    fn new() -> Self {
        Self {
            notes: Mutex::new(vec![]),
        }
    }
}

#[async_trait]
impl NotesRepository for InMemoryNotesRepo {
    async fn save(&self, note: &UserNote) -> Result<(), DomainError> {
        self.notes.lock().await.push(note.clone());
        Ok(())
    }

    async fn find_by_user(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<UserNote>, DomainError> {
        let notes = self.notes.lock().await;
        Ok(notes
            .iter()
            .filter(|n| n.guild_id.as_str() == guild_id && n.user_id.as_str() == user_id)
            .cloned()
            .collect())
    }

    async fn delete(&self, note_id: &str) -> Result<(), DomainError> {
        let uuid = Uuid::parse_str(note_id)
            .map_err(|_| DomainError::NotFound(format!("Note {note_id}")))?;
        let mut notes = self.notes.lock().await;
        notes.retain(|n| n.id != uuid);
        Ok(())
    }

    async fn find_guild_id(&self, note_id: &str) -> Result<Option<String>, DomainError> {
        let uuid = match Uuid::parse_str(note_id) {
            Ok(u) => u,
            Err(_) => return Ok(None),
        };
        let notes = self.notes.lock().await;
        Ok(notes
            .iter()
            .find(|n| n.id == uuid)
            .map(|n| n.guild_id.as_str().to_string()))
    }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_service() -> ManageNotesService {
    let repo = Arc::new(InMemoryNotesRepo::new());
    ManageNotesService::new(repo as Arc<dyn NotesRepository>)
}

fn make_cmd(category: &str) -> AddNoteCommand {
    AddNoteCommand {
        guild_id: "g1".into(),
        user_id: "u1".into(),
        author_id: "mod1".into(),
        author_name: "Bob".into(),
        content: "Test note".into(),
        category: category.into(),
    }
}

// ══════════════════════════════════════════════════════════
// Tests — add_note
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn add_note_saves_to_repo() {
    let svc = build_service();
    let note = svc.add_note(make_cmd("general")).await.unwrap();
    assert_eq!(note.guild_id.as_str(), "g1");
    assert_eq!(note.user_id.as_str(), "u1");
    assert_eq!(note.author_name, "Bob");
    assert_eq!(note.content, "Test note");
    assert_eq!(note.category, "general");
    assert_ne!(note.id, Uuid::nil());
}

#[tokio::test]
async fn add_note_valid_categories() {
    let svc = build_service();
    for cat in &["general", "warning", "positive", "context"] {
        let result = svc.add_note(make_cmd(cat)).await;
        assert!(result.is_ok(), "Category '{}' should be valid", cat);
    }
}

#[tokio::test]
async fn add_note_invalid_category_rejected() {
    let svc = build_service();
    let result = svc.add_note(make_cmd("invalid")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn add_note_preserves_all_fields() {
    let svc = build_service();
    let note = svc
        .add_note(AddNoteCommand {
            guild_id: "g1".into(),
            user_id: "u1".into(),
            author_id: "mod2".into(),
            author_name: "Alice".into(),
            content: "Comportement suspect".into(),
            category: "warning".into(),
        })
        .await
        .unwrap();
    assert_eq!(note.author_id, "mod2");
    assert_eq!(note.author_name, "Alice");
    assert_eq!(note.content, "Comportement suspect");
    assert_eq!(note.category, "warning");
}

// ══════════════════════════════════════════════════════════
// Tests — get_notes
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn get_notes_empty() {
    let svc = build_service();
    let notes = svc.get_notes("g1", "u1").await.unwrap();
    assert!(notes.is_empty());
}

#[tokio::test]
async fn get_notes_returns_user_notes() {
    let svc = build_service();
    svc.add_note(make_cmd("general")).await.unwrap();
    svc.add_note(make_cmd("warning")).await.unwrap();
    let notes = svc.get_notes("g1", "u1").await.unwrap();
    assert_eq!(notes.len(), 2);
}

#[tokio::test]
async fn get_notes_isolates_users() {
    let svc = build_service();
    svc.add_note(make_cmd("general")).await.unwrap();
    let notes = svc.get_notes("g1", "other_user").await.unwrap();
    assert!(notes.is_empty());
}

#[tokio::test]
async fn get_notes_isolates_guilds() {
    let svc = build_service();
    svc.add_note(make_cmd("general")).await.unwrap();
    let notes = svc.get_notes("other_guild", "u1").await.unwrap();
    assert!(notes.is_empty());
}

// ══════════════════════════════════════════════════════════
// Tests — delete_note
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn delete_note_removes_it() {
    let svc = build_service();
    let note = svc.add_note(make_cmd("general")).await.unwrap();
    svc.delete_note(&note.id.to_string()).await.unwrap();
    let notes = svc.get_notes("g1", "u1").await.unwrap();
    assert!(notes.is_empty());
}

#[tokio::test]
async fn delete_note_only_removes_targeted() {
    let svc = build_service();
    let note1 = svc.add_note(make_cmd("general")).await.unwrap();
    svc.add_note(make_cmd("warning")).await.unwrap();
    svc.delete_note(&note1.id.to_string()).await.unwrap();
    let notes = svc.get_notes("g1", "u1").await.unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].category, "warning");
}
