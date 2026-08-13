//! Vol de coins entre membres.
//!
//! Toutes les valeurs qui font l'equilibre — chance de reussite, part volee,
//! penalite d'echec, solde minimum d'une cible — viennent desormais de la
//! configuration du serveur. Elles etaient en dur : regler un vol trop
//! punitif demandait de recompiler, autrement dit de ne jamais le regler.
//!
//! Les defauts de `CoussinConfig` reproduisent exactement les anciennes
//! constantes, donc rien ne change tant que personne ne touche a la
//! configuration.

use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;

use crate::nexus::application::economy_config::load_coussin;
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::{
    inbound::coussin_steal::{CoussinStealUseCase, StealResult},
    outbound::{
        coussin_steal_repository::CoussinStealRepository,
        system::bot_config_repository::BotConfigRepository,
    },
};

pub struct CoussinStealService {
    repo: Arc<dyn CoussinStealRepository>,
    config_repo: Arc<dyn BotConfigRepository>,
}

impl CoussinStealService {
    pub fn new(
        repo: Arc<dyn CoussinStealRepository>,
        config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self { repo, config_repo }
    }
}

#[async_trait]
impl CoussinStealUseCase for CoussinStealService {
    async fn steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        is_piegeur: bool,
    ) -> Result<StealResult, DomainError> {
        if thief_id == victim_id {
            return Err(DomainError::Validation(
                "impossible de se voler soi-meme".into(),
            ));
        }

        let cfg = load_coussin(&self.config_repo, guild_id).await?;
        cfg.ensure_enabled()?;
        if !cfg.steal_enabled {
            return Err(DomainError::Validation(
                "les vols sont desactives sur ce serveur".into(),
            ));
        }

        let (thief, victim) = self.repo.balances(guild_id, thief_id, victim_id).await?;

        // Plancher de pauvrete : sans lui, on peut achever quelqu'un qui n'a
        // deja plus rien. Ca ne rapporte presque rien et ca degoute.
        if victim < cfg.steal_min_victim_coins {
            return Err(DomainError::Validation(format!(
                "cible trop pauvre (moins de {} coins)",
                cfg.steal_min_victim_coins
            )));
        }

        let success = rand::thread_rng().gen_range(0..100) < cfg.steal_chance(is_piegeur);
        let amount = if success {
            cfg.steal_gain(victim)
        } else {
            cfg.steal_penalty(thief)
        };

        self.repo
            .transfer(
                guild_id,
                thief_id,
                victim_id,
                amount,
                success,
                cfg.steal_cooldown_minutes,
            )
            .await?;

        Ok(StealResult { success, amount })
    }
}

#[cfg(test)]
#[path = "tests/coussin_steal_service.rs"]
mod tests;
