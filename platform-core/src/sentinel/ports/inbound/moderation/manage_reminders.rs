use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::moderation::action::sanction_reminder::SanctionReminder;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::errors::DomainError;

pub struct CreateReminderCommand {
    pub guild_id: GuildId,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub action_id: Uuid,
    pub duration_secs: u64,
    pub remind_before_secs: u64,
}

#[async_trait]
pub trait ManageRemindersUseCase: Send + Sync {
    async fn create_reminder(
        &self,
        cmd: CreateReminderCommand,
    ) -> Result<SanctionReminder, DomainError>;
    async fn get_pending_reminders(&self) -> Result<Vec<SanctionReminder>, DomainError>;
    async fn mark_sent(&self, reminder_id: Uuid) -> Result<(), DomainError>;
    async fn cancel_for_action(&self, action_id: Uuid) -> Result<(), DomainError>;
    /// Annule les rappels de ban temporaire actifs pour un utilisateur (unban
    /// manuel precoce, cf. BUG #2). Default no-op pour les stubs de test.
    async fn cancel_for_target(
        &self,
        _guild_id: &str,
        _target_id: &str,
    ) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<SanctionReminder>, DomainError>;
}
