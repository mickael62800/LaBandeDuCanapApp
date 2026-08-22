use super::*;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::sentinel::application::community::manage_polls_service::ManagePollsService;
use crate::sentinel::domain::entities::community::poll::{Poll, PollOption};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_polls::{ManagePollsUseCase, UpsertPollCommand};
use crate::sentinel::ports::outbound::community::poll_repository::PollRepository;

#[derive(Default)]
struct MockPollRepo {
    polls: Mutex<Vec<Poll>>,
    votes: Mutex<Vec<(Uuid, Uuid, String)>>,
}

#[async_trait]
impl PollRepository for MockPollRepo {
    async fn list(&self, _guild_id: &str, _open_only: bool, _limit: i64) -> Result<Vec<Poll>, DomainError> {
        Ok(self.polls.lock().await.clone())
    }
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Poll>, DomainError> {
        Ok(self.polls.lock().await.iter().find(|p| p.id == id).cloned())
    }
    async fn create(&self, cmd: &UpsertPollCommand) -> Result<Poll, DomainError> {
        Ok(Poll {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            question: cmd.question.clone(),
            description: cmd.description.clone(),
            closes_at: cmd.closes_at,
            is_closed: false,
            is_public: cmd.is_public,
            created_by: cmd.created_by.clone(),
            created_at: Utc::now(),
            options: vec![],
        })
    }
    async fn set_closed(&self, id: Uuid, closed: bool) -> Result<bool, DomainError> {
        let mut polls = self.polls.lock().await;
        if let Some(p) = polls.iter_mut().find(|p| p.id == id) {
            p.is_closed = closed;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    async fn delete(&self, id: Uuid) -> Result<bool, DomainError> {
        let mut polls = self.polls.lock().await;
        let len_before = polls.len();
        polls.retain(|p| p.id != id);
        Ok(polls.len() < len_before)
    }
    async fn cast_vote(&self, poll_id: Uuid, option_id: Uuid, user_id: &str) -> Result<bool, DomainError> {
        self.votes.lock().await.push((poll_id, option_id, user_id.into()));
        Ok(true)
    }
    async fn vote_of(&self, poll_id: Uuid, user_id: &str) -> Result<Option<Uuid>, DomainError> {
        Ok(self.votes.lock().await.iter()
            .find(|v| v.0 == poll_id && v.2 == user_id)
            .map(|v| v.1))
    }
}

#[tokio::test]
async fn create_poll_valid() {
    let svc = ManagePollsService::new(Arc::new(MockPollRepo::default()));
    let cmd = UpsertPollCommand {
        guild_id: "g1".into(),
        question: "Question?".into(),
        description: Some("Description".into()),
        closes_at: Utc::now() + Duration::days(1),
        is_public: true,
        created_by: "user1".into(),
        options: vec![("Option A".into(), None), ("Option B".into(), None)],
    };
    let result = svc.create(cmd).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn list_polls() {
    let repo = Arc::new(MockPollRepo::default());
    let svc = ManagePollsService::new(repo.clone());
    let result = svc.list("g1", false, 10).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn close_nonexistent_poll() {
    let svc = ManagePollsService::new(Arc::new(MockPollRepo::default()));
    let result = svc.close(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn delete_poll() {
    let repo = Arc::new(MockPollRepo::default());
    let svc = ManagePollsService::new(repo);
    let id = Uuid::new_v4();
    let result = svc.delete(id).await;
    assert!(result.is_ok());
}
