//! `state` CSRF du flux OAuth2, à usage unique et à durée de vie courte.

use async_trait::async_trait;

use crate::domain::errors::DomainError;

#[async_trait]
pub trait LoginStateStore: Send + Sync {
    /// Enregistre un `state` fraîchement généré, avec son TTL.
    async fn put(&self, state: &str, ttl_secs: u64) -> Result<(), DomainError>;

    /// Consomme un `state` : `true` s'il existait, `false` sinon.
    ///
    /// **Doit être atomique** (un `GETDEL`, pas un `GET` puis un `DEL`) : deux
    /// callbacks concurrents portant le même `state` ne doivent pas réussir
    /// tous les deux. C'est ce qui fait du `state` une protection contre le
    /// rejeu et pas seulement contre le CSRF.
    async fn take(&self, state: &str) -> Result<bool, DomainError>;
}
