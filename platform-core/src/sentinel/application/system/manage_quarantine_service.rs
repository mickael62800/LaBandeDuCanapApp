//! Use case Quarantaine : lit le reglage de la guilde, calcule la date
//! d'expulsion et delegue la persistance au repo. Toute la regle metier vit
//! ici ; le SQL dans `QuarantineRepository`, le handler HTTP ne fait que
//! parser/RBAC/mapper.

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::entities::system::bot_names::SECURITY_BOT;
use crate::sentinel::domain::entities::system::quarantine::{ActiveQuarantine, QuarantineSettings};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::system::manage_quarantine::ManageQuarantineUseCase;
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::sentinel::ports::outbound::system::quarantine_repository::QuarantineRepository;

pub struct ManageQuarantineService {
    repo: Arc<dyn QuarantineRepository>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl ManageQuarantineService {
    pub fn new(
        repo: Arc<dyn QuarantineRepository>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self {
            repo,
            bot_config_repo,
        }
    }
}

#[async_trait]
impl ManageQuarantineUseCase for ManageQuarantineService {
    async fn settings(&self, guild_id: &str) -> Result<QuarantineSettings, DomainError> {
        // Config illisible : on retombe sur les defauts plutot que d'echouer.
        // Une quarantaine qui ne s'enregistre pas laisserait un compte suspect
        // sans echeance, donc jamais expulse — l'inverse du but recherche.
        let configs = self
            .bot_config_repo
            .get_config(guild_id, SECURITY_BOT)
            .await
            .unwrap_or_default();
        let defauts = QuarantineSettings::default();
        let brut = |cle: &str| {
            configs
                .iter()
                .find(|c| c.config_key == cle)
                .map(|c| c.config_value.as_str())
        };
        Ok(QuarantineSettings {
            timeout_secs: brut("quarantine_timeout_secs")
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(defauts.timeout_secs),
            kick_enabled: brut("quarantine_kick_enabled")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(defauts.kick_enabled),
            reminder_secs: brut("quarantine_reminder_secs")
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(defauts.reminder_secs),
            rules_channel_id: brut("rules_channel_id").map(str::to_string),
        }
        .sanitized())
    }

    async fn quarantine_user(
        &self,
        guild_id: &str,
        user_id: &str,
        timeout_secs: Option<i64>,
    ) -> Result<QuarantineSettings, DomainError> {
        let mut settings = self.settings(guild_id).await?;
        // Un appelant peut imposer son delai (outil d'administration). Le cas
        // normal est l'absence de valeur : le reglage de la guilde fait foi, et
        // le bot n'a pas a le connaitre pour poser une quarantaine.
        if let Some(explicite) = timeout_secs.filter(|v| *v > 0) {
            settings = QuarantineSettings {
                timeout_secs: explicite,
                ..settings
            }
            .sanitized();
        }
        let expires_at = settings.expires_from(chrono::Utc::now());
        self.repo.upsert(guild_id, user_id, expires_at).await?;
        Ok(settings)
    }

    async fn list_active(&self) -> Result<Vec<ActiveQuarantine>, DomainError> {
        self.repo.list_active().await
    }

    async fn lift(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.repo.delete(guild_id, user_id).await
    }
}
