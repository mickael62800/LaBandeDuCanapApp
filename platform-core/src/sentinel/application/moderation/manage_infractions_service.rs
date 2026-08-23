use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::entities::moderation::infraction::Infraction;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::manage_infractions::InfractionFilters;
use crate::sentinel::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use crate::sentinel::ports::inbound::moderation::manage_infractions::UserInfractionCounts;
use crate::sentinel::ports::outbound::moderation::infraction_repository::InfractionRepository;

pub struct ManageInfractionsService {
    infraction_repo: Arc<dyn InfractionRepository>,
}

impl ManageInfractionsService {
    pub fn new(infraction_repo: Arc<dyn InfractionRepository>) -> Self {
        Self { infraction_repo }
    }
}

#[async_trait]
impl ManageInfractionsUseCase for ManageInfractionsService {
    async fn count_user_infractions(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UserInfractionCounts, DomainError> {
        let rows = self
            .infraction_repo
            .count_by_action_for_user(guild_id, user_id)
            .await?;

        let mut counts = UserInfractionCounts::default();
        for (action, n) in rows {
            let n = n as u32;
            counts.total = counts.total.saturating_add(n);
            // Les natures non listees alimentent `total` uniquement : le
            // detail affiche reste celui des quatre sanctions courantes.
            match action.as_str() {
                "warn" => counts.warns = n,
                "delete" => counts.deletes = n,
                "mute" => counts.mutes = n,
                "ban" => counts.bans = n,
                _ => {}
            }
        }
        Ok(counts)
    }

    async fn list_infractions(
        &self,
        guild_id: &str,
        filters: InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError> {
        self.infraction_repo.find_by_guild(guild_id, &filters).await
    }

    async fn list_all_infractions(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Infraction>, DomainError> {
        self.infraction_repo.find_all(limit, offset).await
    }

    async fn count_today(&self) -> Result<u64, DomainError> {
        self.infraction_repo.count_today().await
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Infraction>, DomainError> {
        self.infraction_repo.find_by_id(id).await
    }

    async fn delete_infraction(&self, id: &str) -> Result<bool, DomainError> {
        self.infraction_repo.delete_by_id(id).await
    }

    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError> {
        self.infraction_repo
            .delete_older_than_days(guild_id, days)
            .await
    }
}
