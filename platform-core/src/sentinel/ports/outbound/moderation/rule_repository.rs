use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::system::rule::Rule;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait RuleRepository: Send + Sync {
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<Rule>, DomainError>;
    async fn find_all(&self) -> Result<Vec<Rule>, DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Rule>, DomainError>;
    async fn save(&self, rule: &Rule) -> Result<Rule, DomainError>;
    async fn toggle(&self, id: Uuid, enabled: bool) -> Result<(), DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    /// Insere les regles par defaut d'une guild de maniere idempotente
    /// (`ON CONFLICT (guild_id, flag_type) DO NOTHING`) : ne touche pas aux
    /// regles deja presentes (eventuellement modifiees par l'admin).
    async fn seed_defaults(&self, rules: &[Rule]) -> Result<(), DomainError>;
}
