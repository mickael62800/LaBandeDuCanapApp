use async_trait::async_trait;

use crate::atrium::domain::{GameAnnouncement, GameAnnouncementError, GameAnnouncementRequest};

/// Redige l'annonce d'ouverture d'une session de jeu.
///
/// Echoue plutot que de servir un texte de secours : l'annonce PRECEDE le
/// panneau d'inscription, et ouvrir une session sur un message que personne
/// n'a voulu serait pire que de retarder l'ouverture.
#[async_trait]
pub trait GenerateGameAnnouncementUseCase: Send + Sync {
    async fn announce(
        &self,
        request: GameAnnouncementRequest,
    ) -> Result<GameAnnouncement, GameAnnouncementError>;
}
