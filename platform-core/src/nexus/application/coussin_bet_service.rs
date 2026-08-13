//! Paris sur les combats.
//!
//! Les bornes viennent de la configuration du serveur : un pari minimum trop
//! bas noie le salon de mises symboliques, un gain trop genereux vide le jeu
//! de ses combats — il devient plus rentable de parier que de se battre.

use std::sync::Arc;

use async_trait::async_trait;

use crate::nexus::application::economy_config::load_coussin;
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::{
    inbound::coussin_bet::CoussinBetUseCase,
    outbound::{
        coussin_bet_repository::CoussinBetRepository,
        system::bot_config_repository::BotConfigRepository,
    },
};

pub struct CoussinBetService {
    repo: Arc<dyn CoussinBetRepository>,
    config_repo: Arc<dyn BotConfigRepository>,
    cooldowns: Arc<
        dyn crate::nexus::ports::outbound::coussin_cooldown_repository::CoussinCooldownRepository,
    >,
}

impl CoussinBetService {
    pub fn new(
        repo: Arc<dyn CoussinBetRepository>,
        config_repo: Arc<dyn BotConfigRepository>,
        cooldowns: Arc<
            dyn crate::nexus::ports::outbound::coussin_cooldown_repository::CoussinCooldownRepository,
        >,
    ) -> Self {
        Self {
            repo,
            config_repo,
            cooldowns,
        }
    }
}

#[async_trait]
impl CoussinBetUseCase for CoussinBetService {
    async fn place(
        &self,
        guild: &str,
        combat: uuid::Uuid,
        bettor: &str,
        name: &str,
        backed: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        let cfg = load_coussin(&self.config_repo, guild).await?;
        cfg.ensure_enabled()?;
        if !cfg.bet_enabled {
            return Err(DomainError::Validation(
                "les paris sont desactives sur ce serveur".into(),
            ));
        }
        if amount < cfg.bet_min {
            return Err(DomainError::Validation(format!(
                "le pari minimum est de {} coins",
                cfg.bet_min
            )));
        }

        crate::nexus::application::economy_config::ensure_cooldown_over(
            &self.cooldowns,
            guild,
            bettor,
            "bet",
            "tu viens de parier",
        )
        .await?;

        self.repo
            .place(guild, combat, bettor, name, backed, amount)
            .await?;
        self.cooldowns
            .arm(guild, bettor, "bet", cfg.bet_cooldown_minutes)
            .await
    }
}

#[cfg(test)]
#[path = "tests/coussin_bet_service.rs"]
mod tests;
