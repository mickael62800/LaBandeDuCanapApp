use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::user_activity::UserActivity;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait UserActivityRepository: Send + Sync {
    async fn create(&self, activity: &UserActivity) -> Result<(), DomainError>;
    async fn list(
        &self,
        guild_id: &str,
        user_id: &str,
        event_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserActivity>, DomainError>;

    /// Recherche le `message_sent` correspondant a un `message_id` Discord.
    /// Utilise par le bot lors d'un edit pour retrouver l'ancien contenu si
    /// son cache RAM ne l'a pas. Default impl `Ok(None)` pour preserver
    /// les mocks existants.
    async fn find_by_message_id(
        &self,
        _guild_id: &str,
        _message_id: &str,
    ) -> Result<Option<UserActivity>, DomainError> {
        Ok(None)
    }
}
