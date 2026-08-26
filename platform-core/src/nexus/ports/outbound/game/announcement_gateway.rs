use async_trait::async_trait;

/// Faits d'une session de jeu, tels que Nexus les connait.
///
/// Nexus les rassemble ; un autre domaine les met en phrase. Aucune plume ici :
/// c'est ce qui evite deux verites sur ce qu'est une soiree de jeu.
#[derive(Debug, Clone)]
pub struct SessionFacts {
    pub guild_id: String,
    pub game_name: String,
    pub server_name: String,
    pub max_players: Option<u32>,
    /// Ouverture prevue, DEJA mise en forme dans le fuseau de la guilde.
    ///
    /// Nexus formate la date parce que lui seul connait le fuseau et les
    /// plages. La recalculer ailleurs creerait une seconde verite sur l'heure
    /// d'ouverture, et rien ne garantirait qu'elles concordent.
    pub opening_label: Option<String>,
    pub schedule_label: Option<String>,
    /// Reglement de la soiree. Transmis comme contexte, jamais reformule.
    pub rules: Option<String>,
}

/// Pourquoi la redaction a echoue.
///
/// LA DISTINCTION PORTE LA REPRISE. `Indisponible` veut dire « retente plus
/// tard » ; `Refusee` veut dire « ne retente pas, ca ne passera jamais ».
/// `DomainError` ne separe pas les deux — `Infrastructure` et `Internal` s'y
/// confondent — et une reprise batie dessus tournerait en boucle sur une
/// demande definitivement invalide.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AnnouncementError {
    /// Panne, quota epuise, reponse vide : reessayer a un sens.
    #[error("Atrium ne peut pas rediger l'annonce pour l'instant")]
    Indisponible,
    /// Demande mal formee : reessayer n'a aucun sens.
    #[error("annonce refusee : {0}")]
    Refusee(String),
}

/// Redaction de l'annonce d'ouverture, confiee au domaine qui tient la plume.
#[async_trait]
pub trait GameAnnouncementGateway: Send + Sync {
    async fn rediger(&self, faits: SessionFacts) -> Result<String, AnnouncementError>;
}
