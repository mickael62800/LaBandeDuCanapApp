use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::security_event::SecurityEvent;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait SecurityEventRepository: Send + Sync {
    async fn save(&self, event: &SecurityEvent) -> Result<(), DomainError>;
    async fn find_all(&self) -> Result<Vec<SecurityEvent>, DomainError>;
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<SecurityEvent>, DomainError>;
    /// Purge les evenements de securite d'une guilde (+ les manual_watched_users
    /// crees automatiquement par ces evenements). Renvoie (nb_events, nb_watches).
    async fn purge_guild(&self, guild_id: &str) -> Result<(u64, u64), DomainError>;
}
