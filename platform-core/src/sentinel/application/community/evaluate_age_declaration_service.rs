//! Service application — evaluation de la declaration d'age.
//!
//! Lit la config welcome du serveur (via le port outbound) puis applique la
//! regle metier pure `decide_age_check`. Core pur : aucune dependance Discord.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use crate::sentinel::domain::entities::community::age_check::{decide_age_check, AgeCheckDecision};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::evaluate_age_declaration::EvaluateAgeDeclarationUseCase;
use crate::sentinel::ports::outbound::community::welcome_config_repository::WelcomeConfigRepository;

pub struct EvaluateAgeDeclarationService {
    welcome_config_repo: Arc<dyn WelcomeConfigRepository>,
}

impl EvaluateAgeDeclarationService {
    pub fn new(welcome_config_repo: Arc<dyn WelcomeConfigRepository>) -> Self {
        Self {
            welcome_config_repo,
        }
    }
}

#[async_trait]
impl EvaluateAgeDeclarationUseCase for EvaluateAgeDeclarationService {
    async fn evaluate(
        &self,
        guild_id: &str,
        _user_id: &str,
        declared_age: i32,
    ) -> Result<AgeCheckDecision, DomainError> {
        crate::sentinel::application::validation::validate_guild_id(guild_id)?;
        let cfg = self.welcome_config_repo.get_config(guild_id).await?;
        Ok(decide_age_check(
            declared_age,
            cfg.age_minimum,
            cfg.age_ban_days_per_year,
            Utc::now(),
        ))
    }
}
