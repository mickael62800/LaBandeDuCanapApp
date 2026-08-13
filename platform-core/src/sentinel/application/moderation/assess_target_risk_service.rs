//! Service application — evaluation server-side du risque d'une cible.
//!
//! Lit le seuil serveur (`risk_recent_account_days` du bot `moderation-bot`,
//! defaut 7 jours) via le port outbound config, puis applique la regle metier
//! PURE `decide_target_risk`. Core pur : aucune dependance Discord (le bot
//! resout les faits et les passe dans la commande).

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::entities::moderation::target_risk::{
    decide_target_risk, TargetRiskDecision, TargetRiskFacts,
};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::assess_target_risk::{
    AssessTargetRiskCommand, AssessTargetRiskUseCase,
};
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;

/// Nom du bot portant la config moderation (cle `risk_recent_account_days`).
const MODERATION_BOT: &str = "moderation-bot";
/// Cle de config du seuil d'age "compte recent".
const RISK_RECENT_ACCOUNT_DAYS_KEY: &str = "risk_recent_account_days";
/// Defaut historique cote bot : 7 jours.
const DEFAULT_RECENT_ACCOUNT_DAYS: i64 = 7;

pub struct AssessTargetRiskService {
    bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl AssessTargetRiskService {
    pub fn new(bot_config_repo: Arc<dyn BotConfigRepository>) -> Self {
        Self { bot_config_repo }
    }

    /// Lit le seuil "compte recent" depuis la config serveur (defaut 7j).
    async fn recent_account_days(&self, guild_id: &str) -> i64 {
        match self
            .bot_config_repo
            .get_config(guild_id, MODERATION_BOT)
            .await
        {
            Ok(entries) => entries
                .iter()
                .find(|e| e.config_key == RISK_RECENT_ACCOUNT_DAYS_KEY)
                .and_then(|e| e.config_value.parse::<i64>().ok())
                .filter(|&d| d >= 0)
                .unwrap_or(DEFAULT_RECENT_ACCOUNT_DAYS),
            Err(_) => DEFAULT_RECENT_ACCOUNT_DAYS,
        }
    }
}

#[async_trait]
impl AssessTargetRiskUseCase for AssessTargetRiskService {
    async fn assess(
        &self,
        cmd: AssessTargetRiskCommand,
    ) -> Result<TargetRiskDecision, DomainError> {
        crate::sentinel::application::validation::validate_guild_id(&cmd.guild_id)?;
        let threshold = self.recent_account_days(&cmd.guild_id).await;
        let facts = TargetRiskFacts {
            account_age_days: cmd.account_age_days,
            is_bot: cmd.is_bot,
            has_mod_perms: cmd.has_mod_perms,
        };
        Ok(decide_target_risk(&facts, threshold))
    }
}

#[cfg(test)]
#[path = "tests/assess_target_risk.rs"]
mod tests;
