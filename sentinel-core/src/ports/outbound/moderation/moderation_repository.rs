use async_trait::async_trait;

use crate::domain::entities::moderation::action::applied::ModerationAction;
use crate::domain::entities::moderation::action::reversal::ActionReversalInfo;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ModerationRepository: Send + Sync {
    async fn save(&self, action: &ModerationAction) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<ModerationAction>, DomainError>;
    async fn find_by_target(
        &self,
        guild_id: &str,
        target_id: &str,
        limit: i64,
    ) -> Result<Vec<ModerationAction>, DomainError>;
    async fn find_bans(
        &self,
        guild_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ModerationAction>, DomainError>;
    /// Liste toutes les actions de moderation (warn, mute, ban, unban, etc.)
    /// pour une guild (ou toutes si guild_id = None). Utilise pour le journal
    /// unifie du panneau admin.
    async fn find_all_for_guild(
        &self,
        guild_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<ModerationAction>, DomainError>;
    async fn delete_bans_for_user(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<(), DomainError>;
    async fn delete_action(&self, id: uuid::Uuid) -> Result<bool, DomainError>;

    /// Recupere le guild_id d'une action stockee dans `audit_logs`.
    /// Default : None (pour les mocks de test).
    async fn action_guild_id(&self, _action_id: uuid::Uuid) -> Result<Option<String>, DomainError> {
        Ok(None)
    }

    /// Recupere les infos de reversal depuis `audit_logs` (event_type `mod_*`)
    /// en matchant `details->>'action_id'`. Default : None (mocks de test).
    async fn find_action_for_reversal(
        &self,
        _action_id: uuid::Uuid,
    ) -> Result<Option<ActionReversalInfo>, DomainError> {
        Ok(None)
    }

    /// Nombre d'actions posees par un moderateur sur la fenetre (quota
    /// anti-modo compromis). Default : 0 (mocks de test).
    async fn count_recent_mod_actions(
        &self,
        _guild_id: &str,
        _moderator_id: &str,
        _window_secs: i64,
    ) -> Result<i64, DomainError> {
        Ok(0)
    }
}
