use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::moderation::action::sanction_reminder::SanctionReminder;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ReminderRepository: Send + Sync {
    async fn save(&self, reminder: &SanctionReminder) -> Result<(), DomainError>;
    async fn find_pending(&self) -> Result<Vec<SanctionReminder>, DomainError>;
    async fn mark_sent(&self, id: Uuid) -> Result<(), DomainError>;
    async fn cancel_for_action(&self, action_id: Uuid) -> Result<(), DomainError>;
    /// Annule les rappels de ban temporaire encore actifs (`unban_status =
    /// 'pending'`) pour un utilisateur donne. Utilise lors d'un unban manuel
    /// precoce (guild + target connus, action_id inconnu) pour empecher le
    /// worker d'auto-unban d'emettre un `sanction_expired_unban` tardif qui
    /// pourrait lever un ban plus recent applique entre-temps (BUG #2).
    /// Retourne le nombre de rappels annules.
    async fn cancel_for_target(&self, guild_id: &str, target_id: &str) -> Result<u64, DomainError>;
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<SanctionReminder>, DomainError>;
}
