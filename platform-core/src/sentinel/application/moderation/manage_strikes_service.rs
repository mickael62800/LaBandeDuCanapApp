use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::moderation::action::strikes::escalation_for;
use crate::sentinel::domain::entities::moderation::action::strikes::StrikeConfig;
use crate::sentinel::domain::entities::moderation::action::strikes::StrikeResult;
use crate::sentinel::domain::entities::moderation::action::strikes::UserStrike;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::manage_strikes::AddStrikeCommand;
use crate::sentinel::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase;
use crate::sentinel::ports::inbound::moderation::manage_strikes::SaveStrikeConfigCommand;
use crate::sentinel::ports::outbound::moderation::strike_repository::StrikeRepository;

pub struct ManageStrikesService {
    repo: Arc<dyn StrikeRepository>,
}

impl ManageStrikesService {
    pub fn new(repo: Arc<dyn StrikeRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageStrikesUseCase for ManageStrikesService {
    async fn add_strike(&self, cmd: AddStrikeCommand) -> Result<StrikeResult, DomainError> {
        let config = self.get_config(&cmd.guild_id).await?;

        let expires_at = if config.window_secs > 0 {
            Some(Utc::now() + Duration::seconds(config.window_secs))
        } else {
            None
        };

        let strike = UserStrike {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            user_id: cmd.user_id.clone(),
            reason: cmd.reason,
            source: cmd.source,
            infraction_id: cmd.infraction_id,
            expires_at,
            created_at: Utc::now(),
        };

        self.repo.save_strike(&strike).await?;

        if !config.enabled {
            return Ok(StrikeResult {
                strike,
                active_count: 1,
                escalation_action: None,
                escalation_duration: None,
            });
        }

        let active = self
            .repo
            .find_active_strikes(&cmd.guild_id, &cmd.user_id, config.window_secs)
            .await?;
        let active_count = active.len() as u32;

        // N'escalade que si CE strike FRANCHIT un nouveau palier : on compare le
        // palier au compte courant a celui du compte precedent. S'ils sont
        // identiques (deja au-dessus du seuil), pas de re-application -> plus de
        // re-mute/re-ban a chaque strike suivant. Prend aussi en compte un
        // changement de duree entre deux paliers de meme action.
        let current = escalation_for(&config.thresholds, active_count);
        let previous = active_count
            .checked_sub(1)
            .and_then(|prev| escalation_for(&config.thresholds, prev));
        let (escalation_action, escalation_duration) = if current != previous {
            match current {
                Some((action, duration)) => (Some(action), duration),
                None => (None, None),
            }
        } else {
            (None, None)
        };

        Ok(StrikeResult {
            strike,
            active_count,
            escalation_action,
            escalation_duration,
        })
    }

    async fn get_active_strikes(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<UserStrike>, DomainError> {
        let config = self.get_config(guild_id).await?;
        self.repo
            .find_active_strikes(guild_id, user_id, config.window_secs)
            .await
    }

    async fn reset_strikes(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.repo.delete_strikes(guild_id, user_id).await
    }

    async fn get_config(&self, guild_id: &str) -> Result<StrikeConfig, DomainError> {
        match self.repo.get_config(guild_id).await? {
            Some(config) => Ok(config),
            None => Ok(StrikeConfig::default_for_guild(guild_id)),
        }
    }

    async fn save_config(&self, cmd: SaveStrikeConfigCommand) -> Result<StrikeConfig, DomainError> {
        let now = Utc::now();
        let config = StrikeConfig {
            guild_id: cmd.guild_id,
            window_secs: cmd.window_secs,
            thresholds: cmd.thresholds,
            enabled: cmd.enabled,
            created_at: now,
            updated_at: now,
        };
        self.repo.save_config(&config).await?;
        Ok(config)
    }
}
