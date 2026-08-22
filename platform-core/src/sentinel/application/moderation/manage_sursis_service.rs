//! Service : gestion des « bans en sursis ».

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::sentinel::domain::entities::moderation::sursis::{Sursis, SursisStatus};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::manage_sursis::{
    CreateSursisCommand, ManageSursisUseCase,
};
use crate::sentinel::ports::outbound::moderation::sursis_repository::{
    NewSursis, SursisRepository,
};

pub struct ManageSursisService {
    repo: Arc<dyn SursisRepository>,
}

impl ManageSursisService {
    pub fn new(repo: Arc<dyn SursisRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageSursisUseCase for ManageSursisService {
    async fn create(&self, cmd: CreateSursisCommand) -> Result<Sursis, DomainError> {
        let days = cmd.days.clamp(1, 60);
        let expires_at = Utc::now() + Duration::days(days);
        self.repo
            .create(NewSursis {
                guild_id: &cmd.guild_id,
                user_id: &cmd.user_id,
                username: &cmd.username,
                moderator_id: &cmd.moderator_id,
                moderator_name: &cmd.moderator_name,
                reason: &cmd.reason,
                saved_roles: cmd.saved_roles,
                channel_id: cmd.channel_id.as_deref(),
                expires_at,
            })
            .await
    }

    async fn get(&self, id: Uuid) -> Result<Option<Sursis>, DomainError> {
        self.repo.get(id).await
    }

    async fn resolve(&self, id: Uuid, status: SursisStatus) -> Result<bool, DomainError> {
        self.repo.set_status(id, status).await
    }

    async fn list_due(&self) -> Result<Vec<Sursis>, DomainError> {
        self.repo.list_due(Utc::now()).await
    }
}

