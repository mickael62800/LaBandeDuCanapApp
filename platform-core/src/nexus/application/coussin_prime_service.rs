//! Primes posees sur la tete d'un joueur.

use std::sync::Arc;

use async_trait::async_trait;

use crate::nexus::application::economy_config::load_coussin;
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::{
    inbound::coussin_prime::CoussinPrimeUseCase,
    outbound::{
        coussin_prime_repository::CoussinPrimeRepository,
        system::bot_config_repository::BotConfigRepository,
    },
};

pub struct CoussinPrimeService {
    repo: Arc<dyn CoussinPrimeRepository>,
    config_repo: Arc<dyn BotConfigRepository>,
    cooldowns: Arc<
        dyn crate::nexus::ports::outbound::coussin_cooldown_repository::CoussinCooldownRepository,
    >,
}

impl CoussinPrimeService {
    pub fn new(
        repo: Arc<dyn CoussinPrimeRepository>,
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
impl CoussinPrimeUseCase for CoussinPrimeService {
    async fn place(
        &self,
        guild_id: &str,
        target_id: &str,
        target_name: &str,
        placer_id: &str,
        placer_name: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        // Regle universelle, independante de toute configuration : se mettre
        // une prime sur la tete n'a aucun sens.
        if target_id == placer_id {
            return Err(DomainError::Validation(
                "impossible de poser une prime sur soi".into(),
            ));
        }

        let cfg = load_coussin(&self.config_repo, guild_id).await?;
        cfg.ensure_enabled()?;
        if !cfg.prime_enabled {
            return Err(DomainError::Validation(
                "les primes sont desactivees sur ce serveur".into(),
            ));
        }
        if amount < cfg.prime_min {
            return Err(DomainError::Validation(format!(
                "la prime minimum est de {} coins",
                cfg.prime_min
            )));
        }
        // 0 = pas de plafond.
        if cfg.prime_max > 0 && amount > cfg.prime_max {
            return Err(DomainError::Validation(format!(
                "la prime maximum est de {} coins",
                cfg.prime_max
            )));
        }

        crate::nexus::application::economy_config::ensure_cooldown_over(
            &self.cooldowns,
            guild_id,
            placer_id,
            "prime",
            "tu viens de poser un contrat",
        )
        .await?;

        self.repo
            .place(
                guild_id,
                target_id,
                target_name,
                placer_id,
                placer_name,
                amount,
            )
            .await?;
        self.cooldowns
            .arm(guild_id, placer_id, "prime", cfg.prime_cooldown_minutes)
            .await
    }
}

#[cfg(test)]
#[path = "tests/coussin_prime_service.rs"]
mod tests;
