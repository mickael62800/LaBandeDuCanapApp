//! Port outbound : sondes sante systeme (base de donnees).
//!
//! Permet aux handlers de sante (health/info) d'interroger l'etat de la base
//! sans acces direct au pool SQL. Les defaults sont pensés pour les mocks de
//! test : taille inconnue (`-1`) et base consideree comme repondante.

use async_trait::async_trait;

use crate::ops::domain::errors::DomainError;

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

#[cfg(test)]
mod tests {
    use super::*;

    struct DefaultSystemProbe;
    #[async_trait]
    impl SystemProbe for DefaultSystemProbe {}

    struct CustomSystemProbe;
    #[async_trait]
    impl SystemProbe for CustomSystemProbe {
        async fn database_size_bytes(&self) -> Result<i64, DomainError> {
            Ok(1_000_000)
        }

        async fn database_responding(&self) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn default_database_size_is_unknown() {
        let probe = DefaultSystemProbe;
        let size = probe.database_size_bytes().await.unwrap();
        assert_eq!(size, -1);
    }

    #[tokio::test]
    async fn default_database_responding() {
        let probe = DefaultSystemProbe;
        let responding = probe.database_responding().await;
        assert!(responding);
    }

    #[tokio::test]
    async fn custom_database_size() {
        let probe = CustomSystemProbe;
        let size = probe.database_size_bytes().await.unwrap();
        assert_eq!(size, 1_000_000);
    }

    #[tokio::test]
    async fn custom_database_not_responding() {
        let probe = CustomSystemProbe;
        let responding = probe.database_responding().await;
        assert!(!responding);
    }
}
