use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::nexus::application::game::config_loader::load_game_portal_config;
use crate::nexus::domain::entities::game::template::GameTemplate;
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::inbound::game::manage_game_templates::ManageGameTemplatesUseCase;
use crate::nexus::ports::outbound::game::game_template_repository::GameTemplateRepository;
use crate::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository;

pub struct ManageGameTemplatesService {
    repo: Arc<dyn GameTemplateRepository>,
    bot_config: Arc<dyn BotConfigRepository>,
}

impl ManageGameTemplatesService {
    pub fn new(
        repo: Arc<dyn GameTemplateRepository>,
        bot_config: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self { repo, bot_config }
    }
}

#[async_trait]
impl ManageGameTemplatesUseCase for ManageGameTemplatesService {
    async fn list_for_guild(&self, guild_id: &str) -> Result<Vec<GameTemplate>, DomainError> {
        let cfg = load_game_portal_config(&self.bot_config, guild_id).await?;
        let all = self.repo.list().await?;
        Ok(all
            .into_iter()
            .filter(|t| cfg.allowed_templates.iter().any(|s| s == &t.slug))
            .collect())
    }

    async fn get(&self, id: Uuid) -> Result<GameTemplate, DomainError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("template {id} introuvable")))
    }

    async fn get_by_slug(&self, slug: &str) -> Result<GameTemplate, DomainError> {
        self.repo
            .find_by_slug(slug)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("template slug={slug} introuvable")))
    }
}

#[cfg(test)]
#[path = "tests/manage_templates_service.rs"]
mod tests;
