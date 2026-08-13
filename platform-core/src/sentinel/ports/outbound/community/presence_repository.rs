use async_trait::async_trait;

use crate::sentinel::domain::entities::community::presence::{TextChannelActivity, VoicePresence};
use crate::sentinel::domain::errors::DomainError;

/// Lecture de la presence en direct.
///
/// Volontairement en LECTURE SEULE cote API : c'est le bot qui publie, lui
/// seul voit les evenements Discord et connait les permissions des salons.
/// Donner une methode d'ecriture ici inviterait a fabriquer une presence
/// depuis l'API, qui n'a aucun moyen de savoir si elle est vraie.
#[async_trait]
pub trait PresenceRepository: Send + Sync {
    /// Instantane vocal d'une guilde. `None` si le bot n'a rien publie ou si
    /// la cle a expire — cas normal, pas une erreur.
    async fn voice(&self, guild_id: &str) -> Result<Option<VoicePresence>, DomainError>;

    /// Salons ecrits actifs dans la fenetre, les plus recents d'abord.
    async fn text_activity(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<TextChannelActivity>, DomainError>;
}
