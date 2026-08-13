use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

use crate::nexus::domain::errors::DomainError;

#[async_trait]
pub trait GameServerConfigRepository: Send + Sync {
    /// Retourne un map key -> value pour ce serveur (overrides uniquement).
    async fn get_all(&self, server_id: Uuid) -> Result<HashMap<String, String>, DomainError>;

    /// Upsert un override.
    async fn upsert(
        &self,
        server_id: Uuid,
        key: &str,
        value: &str,
        updated_by: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Supprime un override (retour au default du template).
    async fn delete(&self, server_id: Uuid, key: &str) -> Result<(), DomainError>;

    /// Replace tous les overrides d'un serveur (atomique).
    async fn replace_all(
        &self,
        server_id: Uuid,
        entries: HashMap<String, String>,
        updated_by: Option<&str>,
    ) -> Result<(), DomainError>;
}
