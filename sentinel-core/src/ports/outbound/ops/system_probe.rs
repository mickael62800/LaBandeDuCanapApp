//! Port outbound : sondes sante systeme (base de donnees).
//!
//! Permet aux handlers de sante (health/info) d'interroger l'etat de la base
//! sans acces direct au pool SQL. Les defaults sont pensés pour les mocks de
//! test : taille inconnue (`-1`) et base consideree comme repondante.

use async_trait::async_trait;

use crate::domain::errors::DomainError;

#[async_trait]
pub trait SystemProbe: Send + Sync {
    /// Taille de la base de donnees courante en octets. `-1` si inconnue
    /// (convention historique de `/system/info`).
    async fn database_size_bytes(&self) -> Result<i64, DomainError> {
        Ok(-1)
    }

    /// La base de donnees repond-elle (`SELECT 1` cote adapter Postgres) ?
    async fn database_responding(&self) -> bool {
        true
    }
}
