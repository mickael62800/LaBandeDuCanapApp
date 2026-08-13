use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::rule::Rule;
use crate::sentinel::domain::enums::moderation::flag_type::FlagType;
use crate::sentinel::domain::errors::DomainError;

pub struct CreateRuleCommand {
    pub guild_id: GuildId,
    pub flag_type: FlagType,
    pub weight: f64,
    pub threshold_warn: f64,
    pub threshold_delete: f64,
    pub threshold_mute: f64,
    pub threshold_ban: f64,
    pub enabled: bool,
}

#[async_trait]
pub trait ManageRulesUseCase: Send + Sync {
    async fn get_rules(&self, guild_id: &str) -> Result<Vec<Rule>, DomainError>;
    async fn get_all_rules(&self) -> Result<Vec<Rule>, DomainError>;
    async fn toggle_rule(&self, rule_id: Uuid, enabled: bool) -> Result<bool, DomainError>;
    async fn create_or_update_rule(&self, command: CreateRuleCommand) -> Result<Rule, DomainError>;
    async fn delete_rule(&self, guild_id: &str, rule_id: Uuid) -> Result<(), DomainError>;
    /// Seede les regles de moderation par defaut d'une guild (idempotent).
    /// Appele a l'enregistrement de la guild.
    async fn seed_default_rules(&self, guild_id: &str) -> Result<(), DomainError>;
}
