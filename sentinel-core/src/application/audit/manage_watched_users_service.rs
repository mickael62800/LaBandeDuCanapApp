use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::audit::watched_user::WatchedUser;
use crate::domain::errors::DomainError;
use crate::ports::inbound::audit::manage_security::ManageSecurityUseCase;
use crate::ports::inbound::audit::manage_watched_users::ManageWatchedUsersUseCase;
use crate::ports::inbound::audit::manage_watched_users::UserDossier;
use crate::ports::inbound::moderation::manage_infractions::InfractionFilters;
use crate::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use crate::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use crate::ports::outbound::audit::watched_user_repository::WatchedUserRepository;

pub struct ManageWatchedUsersService {
    watched_repo: Arc<dyn WatchedUserRepository>,
    infractions_uc: Arc<dyn ManageInfractionsUseCase>,
    moderation_uc: Arc<dyn ManageModerationUseCase>,
    security_uc: Arc<dyn ManageSecurityUseCase>,
}

impl ManageWatchedUsersService {
    pub fn new(
        watched_repo: Arc<dyn WatchedUserRepository>,
        infractions_uc: Arc<dyn ManageInfractionsUseCase>,
        moderation_uc: Arc<dyn ManageModerationUseCase>,
        security_uc: Arc<dyn ManageSecurityUseCase>,
    ) -> Self {
        Self {
            watched_repo,
            infractions_uc,
            moderation_uc,
            security_uc,
        }
    }
}

#[async_trait]
impl ManageWatchedUsersUseCase for ManageWatchedUsersService {
    async fn list_watched_users(
        &self,
        guild_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WatchedUser>, DomainError> {
        self.watched_repo
            .find_watched_users(guild_id, limit, offset)
            .await
    }

    async fn get_user_dossier(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UserDossier, DomainError> {
        let users = self
            .watched_repo
            .find_watched_users(Some(guild_id), 1000, 0)
            .await?;
        let user = users
            .into_iter()
            .find(|u| u.user_id.as_str() == user_id)
            .ok_or_else(|| DomainError::NotFound(format!("Utilisateur {} introuvable", user_id)))?;

        let filters = InfractionFilters {
            user_id: Some(user_id.to_string()),
            action: None,
            limit: 100,
            offset: 0,
        };
        let infractions = self
            .infractions_uc
            .list_infractions(guild_id, filters)
            .await?;

        let history = self.moderation_uc.get_history(guild_id, user_id).await?;

        let all_events = self.security_uc.list_events(Some(guild_id)).await?;
        let security_events: Vec<_> = all_events
            .into_iter()
            .filter(|e| e.user_ids.contains(&user_id.to_string()))
            .collect();

        let notes = vec![];

        Ok(UserDossier {
            user,
            infractions,
            moderation_actions: history.actions,
            security_events,
            notes,
        })
    }

    async fn add_manual_watch(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        reason: &str,
    ) -> Result<(), DomainError> {
        self.watched_repo
            .add_manual_watch(guild_id, user_id, username, reason, "desktop")
            .await
    }

    async fn remove_manual_watch(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.watched_repo
            .remove_manual_watch(guild_id, user_id)
            .await
    }
}

#[cfg(test)]
#[path = "tests/manage_watched_users.rs"]
mod tests;

