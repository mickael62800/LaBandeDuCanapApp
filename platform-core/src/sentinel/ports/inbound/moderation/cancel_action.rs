//! Annulation d'une action de moderation (`/unwarn`, bouton « annuler » du web).
//!
//! Cette operation n'est pas une simple suppression en base : selon le type
//! d'action elle doit aussi debannir sur Discord, retirer un timeout, et
//! annuler le rappel d'auto-unban encore en attente. Cette orchestration
//! vivait dans le handler HTTP ; elle a ete remontee ici quand le bot est
//! passe en gRPC, pour que les deux adaptateurs partagent le meme
//! comportement au lieu d'en maintenir deux copies.

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;

/// Resultat d'une annulation, du point de vue de l'appelant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelOutcome {
    /// L'action existait et a ete supprimee.
    Cancelled,
    /// Aucune action ne porte cet identifiant.
    NotFound,
}

#[async_trait]
pub trait CancelModerationActionUseCase: Send + Sync {
    /// Annule l'action `action_id` : applique l'effet Discord inverse puis
    /// supprime la ligne.
    ///
    /// Les effets Discord sont **best-effort** : une panne de l'API Discord ne
    /// doit pas empecher la suppression en base, sinon l'interface afficherait
    /// indefiniment une sanction que le moderateur a deja voulu annuler.
    async fn cancel(&self, action_id: uuid::Uuid) -> Result<CancelOutcome, DomainError>;
}
