//! Port outbound de persistance des re-attributions de roles en attente.
//!
//! Table `pending_role_grants` (clef primaire (guild_id, user_id)). `take`
//! s'appuie sur un `DELETE ... RETURNING` pour une lecture-suppression
//! ATOMIQUE (pas de double attribution possible).

use async_trait::async_trait;

use crate::sentinel::domain::entities::guild_backup::pending_role_grant::PendingRoleGrant;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait PendingRoleGrantRepository: Send + Sync {
    /// Upsert d'un lot de grants (par (guild_id, user_id)). Renvoie le nombre
    /// de lignes affectees.
    async fn upsert_many(&self, grants: &[PendingRoleGrant]) -> Result<u64, DomainError>;

    /// Lit ET supprime ATOMIQUEMENT (DELETE ... RETURNING) les role_ids d'un
    /// membre. `None` si aucune ligne.
    async fn take(&self, guild_id: &str, user_id: &str)
        -> Result<Option<Vec<String>>, DomainError>;

    /// Supprime toutes les entrees d'une guild. Renvoie le nombre supprime.
    async fn clear_guild(&self, guild_id: &str) -> Result<u64, DomainError>;
}
