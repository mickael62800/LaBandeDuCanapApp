use async_trait::async_trait;

use crate::sentinel::domain::entities::community::presence::{TextChannelActivity, VoicePresence};
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ReadPresenceUseCase: Send + Sync {
    /// Presence vocale, ou `None` si l'instantane est absent OU perime.
    ///
    /// Le service applique le controle de fraicheur : le repository rend ce
    /// qu'il trouve, la decision « c'est trop vieux pour etre montre »
    /// appartient au metier.
    async fn voice(&self, guild_id: &str) -> Result<Option<VoicePresence>, DomainError>;

    /// Salons ecrits actifs, deja filtres sur la fenetre.
    async fn text_activity(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<TextChannelActivity>, DomainError>;
}
