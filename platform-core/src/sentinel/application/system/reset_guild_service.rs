use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::system::reset_guild::{ResetGuildOutcome, ResetGuildUseCase};
use crate::sentinel::ports::outbound::system::guild_reset_repository::GuildResetRepository;

pub struct ResetGuildService {
    repo: Arc<dyn GuildResetRepository>,
}

impl ResetGuildService {
    pub fn new(repo: Arc<dyn GuildResetRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ResetGuildUseCase for ResetGuildService {
    async fn reset(
        &self,
        guild_id: &str,
        confirmation: &str,
    ) -> Result<ResetGuildOutcome, DomainError> {
        crate::sentinel::application::validation::validate_guild_id(guild_id)?;
        // Confirmation forte : le nom saisi doit correspondre EXACTEMENT au nom
        // du serveur (anti-clic accidentel sur une action irreversible).
        let name = self
            .repo
            .guild_name(guild_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("serveur {guild_id} inconnu")))?;
        if confirmation.trim() != name {
            return Err(DomainError::Forbidden(
                "Confirmation incorrecte : saisis le nom exact du serveur.".into(),
            ));
        }
        // 1. Collecte le contexte Discord AVANT le wipe (sinon les ids sont perdus).
        let discord_context = self.repo.collect_discord_context(guild_id).await?;
        // 2. Efface toutes les donnees du serveur (transaction).
        let tables_wiped = self.repo.wipe_guild(guild_id).await?;
        let total_rows = tables_wiped.iter().map(|(_, n)| *n).sum();
        Ok(ResetGuildOutcome {
            discord_context,
            tables_wiped,
            total_rows,
        })
    }
}

#[cfg(test)]
#[path = "tests/reset_guild.rs"]
mod tests;
