//! Use case persistance bot : delegue la persistance au repo. Endpoints
//! simples fire-and-forget ; le SQL vit dans `BotPersistenceRepository`, le
//! handler HTTP ne fait que parser/valider/mapper.

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::system::manage_bot_persistence::ManageBotPersistenceUseCase;
use crate::sentinel::ports::outbound::system::bot_persistence_repository::BotPersistenceRepository;

pub struct ManageBotPersistenceService {
    repo: Arc<dyn BotPersistenceRepository>,
}

impl ManageBotPersistenceService {
    pub fn new(repo: Arc<dyn BotPersistenceRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageBotPersistenceUseCase for ManageBotPersistenceService {
    async fn update_streak(
        &self,
        guild_id: &str,
        user_id: &str,
        streak_current: i32,
        streak_best: i32,
        streak_last_day: i32,
        streak_last_year: i32,
    ) -> Result<(), DomainError> {
        self.repo
            .update_streak(
                guild_id,
                user_id,
                streak_current,
                streak_best,
                streak_last_day,
                streak_last_year,
            )
            .await
    }
}
