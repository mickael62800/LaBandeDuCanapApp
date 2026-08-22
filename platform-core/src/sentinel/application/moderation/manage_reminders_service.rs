use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::moderation::action::sanction_reminder::SanctionReminder;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::manage_reminders::CreateReminderCommand;
use crate::sentinel::ports::inbound::moderation::manage_reminders::ManageRemindersUseCase;
use crate::sentinel::ports::outbound::moderation::reminder_repository::ReminderRepository;

const DEFAULT_REMIND_BEFORE_SECS: u64 = 3600; // 1 heure avant expiration

pub struct ManageRemindersService {
    repo: Arc<dyn ReminderRepository>,
}

impl ManageRemindersService {
    pub fn new(repo: Arc<dyn ReminderRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageRemindersUseCase for ManageRemindersService {
    async fn create_reminder(
        &self,
        cmd: CreateReminderCommand,
    ) -> Result<SanctionReminder, DomainError> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(cmd.duration_secs as i64);
        let remind_before = if cmd.remind_before_secs > 0 {
            cmd.remind_before_secs
        } else {
            DEFAULT_REMIND_BEFORE_SECS
        };

        // BUG #1/#2 : on cree TOUJOURS la ligne de rappel pour une sanction
        // temporaire — c'est elle qui porte `expires_at` et alimente le job
        // d'auto-unban (colonne `unban_status`, geree par le worker).
        //
        // Le DM "early" au moderateur (status 'pending', consomme par
        // send_reminders) n'a de sens que si la duree depasse `remind_before` ;
        // sinon le rappel serait deja en retard a la creation. Dans ce cas on
        // marque la ligne 'skipped' pour ne PAS envoyer de DM, mais l'unban a
        // l'expiration reste assure (chemin independant).
        let remind_at = expires_at - Duration::seconds(remind_before as i64);
        let status = if cmd.duration_secs <= remind_before {
            "skipped"
        } else {
            "pending"
        };

        let reminder = SanctionReminder {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            moderator_id: cmd.moderator_id,
            moderator_name: cmd.moderator_name,
            target_id: cmd.target_id,
            target_name: cmd.target_name,
            action_type: cmd.action_type,
            reason: cmd.reason,
            action_id: cmd.action_id,
            remind_at,
            expires_at,
            status: status.into(),
            created_at: now,
        };

        self.repo.save(&reminder).await?;
        Ok(reminder)
    }

    async fn get_pending_reminders(&self) -> Result<Vec<SanctionReminder>, DomainError> {
        self.repo.find_pending().await
    }

    async fn mark_sent(&self, reminder_id: Uuid) -> Result<(), DomainError> {
        self.repo.mark_sent(reminder_id).await
    }

    async fn cancel_for_action(&self, action_id: Uuid) -> Result<(), DomainError> {
        self.repo.cancel_for_action(action_id).await
    }

    async fn cancel_for_target(&self, guild_id: &str, target_id: &str) -> Result<u64, DomainError> {
        self.repo.cancel_for_target(guild_id, target_id).await
    }

    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<SanctionReminder>, DomainError> {
        self.repo.find_by_guild(guild_id).await
    }
}

