//! Port inbound : « qui appelle, et a-t-il le droit d'entrer ? »
//!
//! C'est LA question que les trois plateformes posent. Avant, chacune la posait
//! à `sentinel-api` — ou plutôt, nginx la posait pour elles. Elle est
//! maintenant servie par l'identité, pour tout le monde de la même façon.

use async_trait::async_trait;

use crate::domain::entities::identity::AccessVerdict;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ResolveAccessUseCase: Send + Sync {
    /// Résout l'identité derrière un access token et applique la règle d'accès.
    ///
    /// `Err` signifie « impossible de trancher » (Discord injoignable), ce qui
    /// n'est PAS la même chose qu'un refus : l'appelant doit répondre 503 et
    /// non 403. Confondre les deux ferait passer une panne réseau pour une
    /// révocation de droits.
    async fn resolve(&self, access_token: &str) -> Result<AccessVerdict, DomainError>;
}
