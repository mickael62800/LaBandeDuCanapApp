//! Presence en direct.
//!
//! Le service ne fait presque rien — et c'est voulu. La donnee vient du bot,
//! deja filtree aux salons publics. Ce qui reste ici est la seule decision
//! metier : jusqu'a quand un instantane merite-t-il d'etre montre.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use crate::sentinel::domain::entities::community::presence::{TextChannelActivity, VoicePresence};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::read_presence::ReadPresenceUseCase;
use crate::sentinel::ports::outbound::community::presence_repository::PresenceRepository;

const MAX_TEXT_CHANNELS: i64 = 8;

pub struct ReadPresenceService {
    repo: Arc<dyn PresenceRepository>,
}

impl ReadPresenceService {
    pub fn new(repo: Arc<dyn PresenceRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ReadPresenceUseCase for ReadPresenceService {
    async fn voice(&self, guild_id: &str) -> Result<Option<VoicePresence>, DomainError> {
        let now = Utc::now();

        // Un instantane perime est ecarte ici plutot que masque par le front :
        // sinon chaque client (site, bot, une future application) devrait
        // reimplementer le meme seuil, et l'un des trois se tromperait.
        Ok(self.repo.voice(guild_id).await?.filter(|p| p.is_fresh(now)))
    }

    async fn text_activity(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<TextChannelActivity>, DomainError> {
        let now = Utc::now();

        let mut salons = self
            .repo
            .text_activity(guild_id, limit.clamp(1, MAX_TEXT_CHANNELS))
            .await?;

        // Le repository borne sa lecture dans le temps, mais rien ne garantit
        // que l'horloge Redis et celle de l'API concordent : on refiltre.
        salons.retain(|s| s.is_within_window(now));
        Ok(salons)
    }
}

#[cfg(test)]
#[path = "tests/read_presence.rs"]
mod tests;
