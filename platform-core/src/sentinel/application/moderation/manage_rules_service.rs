use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::system::rule::Rule;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::manage_rules::CreateRuleCommand;
use crate::sentinel::ports::inbound::moderation::manage_rules::ManageRulesUseCase;
use tracing::warn;

use crate::sentinel::ports::outbound::moderation::rule_repository::RuleRepository;
use crate::sentinel::ports::outbound::system::cache::CachePort;
pub struct ManageRulesService {
    rule_repo: Arc<dyn RuleRepository>,
    cache: Arc<dyn CachePort>,
}

impl ManageRulesService {
    pub fn new(rule_repo: Arc<dyn RuleRepository>, cache: Arc<dyn CachePort>) -> Self {
        Self { rule_repo, cache }
    }
}

#[async_trait]
impl ManageRulesUseCase for ManageRulesService {
    async fn get_rules(&self, guild_id: &str) -> Result<Vec<Rule>, DomainError> {
        self.rule_repo.find_by_guild(guild_id).await
    }

    async fn get_all_rules(&self) -> Result<Vec<Rule>, DomainError> {
        self.rule_repo.find_all().await
    }

    async fn toggle_rule(&self, rule_id: Uuid, enabled: bool) -> Result<bool, DomainError> {
        self.rule_repo.toggle(rule_id, enabled).await?;

        // Invalider le cache — on ne connaît pas le guild_id ici, invalider par pattern
        if let Some(rule) = self.rule_repo.find_by_id(rule_id).await? {
            if let Err(e) = self.cache.invalidate_rules(&rule.guild_id).await {
                warn!(error = %e, guild_id = %rule.guild_id, "Echec invalidation cache rules");
            }
        }

        Ok(enabled)
    }

    async fn create_or_update_rule(&self, cmd: CreateRuleCommand) -> Result<Rule, DomainError> {
        if cmd.weight < 0.0 {
            return Err(DomainError::ValidationError(
                "Le poids ne peut pas être négatif".into(),
            ));
        }
        if cmd.threshold_warn >= cmd.threshold_delete
            || cmd.threshold_delete >= cmd.threshold_mute
            || cmd.threshold_mute >= cmd.threshold_ban
        {
            return Err(DomainError::ValidationError(
                "Les seuils doivent être croissants : warn < delete < mute < ban".into(),
            ));
        }

        let now = chrono::Utc::now();
        let rule = Rule {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            flag_type: cmd.flag_type,
            weight: cmd.weight,
            threshold_warn: cmd.threshold_warn,
            threshold_delete: cmd.threshold_delete,
            threshold_mute: cmd.threshold_mute,
            threshold_ban: cmd.threshold_ban,
            enabled: cmd.enabled,
            created_at: now,
            updated_at: now,
        };

        let saved = self.rule_repo.save(&rule).await?;

        // Invalider le cache pour ce serveur
        if let Err(e) = self.cache.invalidate_rules(&cmd.guild_id).await {
            warn!(error = %e, guild_id = %cmd.guild_id, "Echec invalidation cache rules");
        }

        Ok(saved)
    }

    async fn delete_rule(&self, guild_id: &str, rule_id: Uuid) -> Result<(), DomainError> {
        self.rule_repo.delete(rule_id).await?;
        if let Err(e) = self.cache.invalidate_rules(guild_id).await {
            warn!(error = %e, guild_id = %guild_id, "Echec invalidation cache rules");
        }
        Ok(())
    }

    async fn seed_default_rules(&self, guild_id: &str) -> Result<(), DomainError> {
        let rules = Rule::default_seed(&guild_id.to_string().into());
        self.rule_repo.seed_defaults(&rules).await?;
        if let Err(e) = self.cache.invalidate_rules(guild_id).await {
            warn!(error = %e, guild_id = %guild_id, "Echec invalidation cache rules");
        }
        Ok(())
    }
}
