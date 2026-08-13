//! Service Community : decisions d'eligibilite (role + parrainage). Lit la
//! config serveur via le port sortant `BotConfigRepository`, puis applique les
//! regles PURES du domaine (`domain::entities::community::eligibility`). Aucune
//! dependance Discord : le bot fournit les donnees (roles, dates de join).

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::entities::community::eligibility::{
    check_prerequisites, days_since, evaluate_sponsorship, parse_prerequisites, EligibilityDecision,
};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::check_eligibility::{
    CheckEligibilityUseCase, CheckRoleEligibilityCommand, ValidateSponsorshipCommand,
};
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;

/// Nom du module de config (aligne sur `MODULE_BOT_NAME` cote bot).
const COMMUNITY_BOT: &str = "community-bot";

pub struct CheckEligibilityService {
    config: Arc<dyn BotConfigRepository>,
}

impl CheckEligibilityService {
    pub fn new(config: Arc<dyn BotConfigRepository>) -> Self {
        Self { config }
    }
}

use crate::sentinel::domain::entities::system::bot_config::{cfg_str, cfg_u64};

/// Horodatage courant (secondes unix). Isole pour la lisibilite/tests.
fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

#[async_trait]
impl CheckEligibilityUseCase for CheckEligibilityService {
    async fn check_role_eligibility(
        &self,
        cmd: CheckRoleEligibilityCommand,
    ) -> Result<EligibilityDecision, DomainError> {
        let cfg = self
            .config
            .get_config(&cmd.guild_id, COMMUNITY_BOT)
            .await
            .unwrap_or_default();

        let raw = cfg_str(&cfg, "role_prerequisites").unwrap_or("");
        let prereqs = parse_prerequisites(raw);

        // `None` => 0 jour (reproduit le `unwrap_or(0)` du bot pour les prereqs).
        let joined_days = cmd
            .joined_at_unix
            .map(|j| days_since(now_unix(), j))
            .unwrap_or(0);

        Ok(check_prerequisites(
            &prereqs,
            cmd.role_id,
            &cmd.user_roles,
            joined_days,
        ))
    }

    async fn validate_sponsorship(
        &self,
        cmd: ValidateSponsorshipCommand,
    ) -> Result<EligibilityDecision, DomainError> {
        let cfg = self
            .config
            .get_config(&cmd.guild_id, COMMUNITY_BOT)
            .await
            .unwrap_or_default();

        let min_parrain_days = cfg_u64(&cfg, "sponsor_min_parrain_days", 7);
        let max_filleul_days = cfg_u64(&cfg, "sponsor_max_filleul_days", 30);

        let now = now_unix();
        // Parrain absent => 0 jour (echoue le min). Filleul absent => u64::MAX
        // (echoue le max). Reproduit exactement les defauts du bot.
        let sponsor_days = cmd
            .sponsor_joined_at_unix
            .map(|j| days_since(now, j))
            .unwrap_or(0);
        let sponsored_days = cmd
            .sponsored_joined_at_unix
            .map(|j| days_since(now, j))
            .unwrap_or(u64::MAX);

        Ok(evaluate_sponsorship(
            cmd.sponsor_id,
            cmd.sponsored_id,
            sponsor_days,
            sponsored_days,
            min_parrain_days,
            max_filleul_days,
        ))
    }
}
