//! Use case inbound de gestion des re-attributions de roles en attente.
//!
//! Cycle de vie : au restore, le bot `save_grants` la map `{user_id ->
//! [nouveau role_id]}` pour TOUS les membres captures. Quand un membre rejoint,
//! le bot `take_grant` (lit ET supprime ATOMIQUEMENT) pour lui re-attribuer ses
//! roles UNE seule fois (idempotence). `clear_guild` purge tout (nouveau
//! restore).

use async_trait::async_trait;

use crate::sentinel::domain::entities::guild_backup::pending_role_grant::PendingRoleGrant;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ManagePendingRoleGrantsUseCase: Send + Sync {
    /// Upsert par (guild_id, user_id) : chaque entree REMPLACE la precedente.
    /// Renvoie le nombre d'entrees ecrites (grants a `role_ids` vide ignores).
    async fn save_grants(
        &self,
        guild_id: &str,
        grants: Vec<PendingRoleGrant>,
    ) -> Result<u64, DomainError>;

    /// Lit ET supprime ATOMIQUEMENT (DELETE ... RETURNING) les roles en attente
    /// d'un membre. `None` si aucune entree (le membre n'est re-role qu'une
    /// fois). Garantit l'idempotence du hook de join.
    async fn take_grant(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Vec<String>>, DomainError>;

    /// Purge toutes les entrees en attente d'une guild. Renvoie le nombre
    /// d'entrees supprimees (ex: repartir propre avant un nouveau restore).
    async fn clear_guild(&self, guild_id: &str) -> Result<u64, DomainError>;
}
