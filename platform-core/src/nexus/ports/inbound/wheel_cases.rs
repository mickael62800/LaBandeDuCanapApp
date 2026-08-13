//! Port inbound : consulter et redefinir les cases de la Roue d'un serveur.

use async_trait::async_trait;

use crate::nexus::domain::entities::wheel::WheelCaseData;
use crate::nexus::domain::errors::DomainError;

/// La roue d'un serveur, telle qu'un editeur doit l'afficher.
#[derive(Debug, Clone)]
pub struct WheelCases {
    pub cases: Vec<WheelCaseData>,
    /// `false` quand ce sont les cases historiques, faute de personnalisation.
    ///
    /// L'editeur en a besoin : sans ce drapeau il ne saurait pas distinguer
    /// « ce serveur a choisi exactement la roue d'origine » de « ce serveur
    /// n'a rien choisi », et ne pourrait pas proposer de revenir en arriere.
    pub customized: bool,
}

#[async_trait]
pub trait ManageWheelCasesUseCase: Send + Sync {
    async fn list(&self, guild_id: &str) -> Result<WheelCases, DomainError>;

    /// Remplace la roue. Une liste VIDE efface la personnalisation et fait
    /// revenir la roue historique — c'est le « annuler mes modifications ».
    async fn replace(
        &self,
        guild_id: &str,
        cases: Vec<WheelCaseData>,
    ) -> Result<WheelCases, DomainError>;
}
