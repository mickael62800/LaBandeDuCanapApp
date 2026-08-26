use async_trait::async_trait;
use uuid::Uuid;

use crate::nexus::ports::outbound::game::announcement_gateway::AnnouncementError;

/// Ce que la reprise doit savoir d'un echec.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionAnnouncementError {
    #[error("serveur {0} introuvable")]
    Introuvable(Uuid),
    /// La session n'attend aucune annonce : elle a deja la sienne, ou elle n'a
    /// pas encore de salon ou la publier.
    #[error("rien a annoncer pour ce serveur")]
    RienAAnnoncer,
    /// Le plafond de tentatives est atteint. On cesse : accumuler des appels
    /// qui echouent tous de la meme facon n'apporte rien et coute un quota.
    #[error("plafond de tentatives atteint")]
    AbandonApresPlafond,
    #[error(transparent)]
    Redaction(#[from] AnnouncementError),
    #[error("erreur interne : {0}")]
    Interne(String),
}

/// Redaction de l'annonce d'ouverture d'une session.
///
/// Le cas d'usage rassemble les faits — jeu, jauge de joueurs, horaires,
/// ouverture prevue — et confie la plume au domaine qui l'a. Il ne publie rien :
/// seul le bot voit Discord.
#[async_trait]
pub trait SessionAnnouncementUseCase: Send + Sync {
    /// Texte a publier avant le panneau d'inscription.
    async fn rediger(&self, server_id: Uuid) -> Result<String, SessionAnnouncementError>;

    /// L'annonce vient d'etre publiee : la session ne doit plus etre reprise.
    async fn marquer_publiee(&self, server_id: Uuid) -> Result<(), SessionAnnouncementError>;
}
