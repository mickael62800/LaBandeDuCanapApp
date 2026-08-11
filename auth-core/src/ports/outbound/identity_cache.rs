//! Cache « access token → discord_user_id ».
//!
//! Sans lui, chaque requête du back-office déclencherait un `GET /users/@me`
//! chez Discord — c'est-à-dire un rate limit atteint en quelques minutes de
//! navigation, et une latence subie sur chaque appel.
//!
//! Le port ne prend JAMAIS l'access token en clair : l'adapter reçoit une clé
//! déjà dérivée. Voir `application::resolve_access` pour la dérivation.

use async_trait::async_trait;

use crate::domain::errors::DomainError;

#[async_trait]
pub trait IdentityCache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, DomainError>;

    /// Best-effort : une panne du cache ne doit pas fermer le back-office.
    /// L'implémentation absorbe ses propres erreurs d'écriture.
    async fn put(&self, key: &str, discord_user_id: &str, ttl_secs: u64);
}
