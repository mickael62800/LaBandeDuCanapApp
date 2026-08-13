use async_trait::async_trait;

use crate::sentinel::domain::entities::system::rule::Rule;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait CachePort: Send + Sync {
    // Rules cache (TTL 5 min)
    async fn get_rules(&self, guild_id: &str) -> Result<Option<Vec<Rule>>, DomainError>;
    async fn set_rules(&self, guild_id: &str, rules: &[Rule]) -> Result<(), DomainError>;
    async fn invalidate_rules(&self, guild_id: &str) -> Result<(), DomainError>;

    // Generic JSON cache (for tickets, stats, moderation history, security events)
    async fn get_json(&self, key: &str) -> Result<Option<String>, DomainError>;
    async fn set_json(&self, key: &str, json: &str, ttl_secs: u64) -> Result<(), DomainError>;
    async fn invalidate(&self, key: &str) -> Result<(), DomainError>;
    async fn invalidate_pattern(&self, pattern: &str) -> Result<(), DomainError>;
}
