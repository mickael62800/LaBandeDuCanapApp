//! Port outbound : reglages d'alerte d'un serveur de jeu.
//!
//! L'URL du webhook est un SECRET — qui l'a peut ecrire dans le salon. Elle ne
//! sort d'ici que vers l'envoyeur, jamais vers une reponse HTTP ni un log.

use async_trait::async_trait;

use crate::nexus::domain::entities::game::alert::{AlertKind, AlertSettings};
use crate::nexus::domain::errors::DomainError;

/// Reglages d'un serveur, accompagnes de leur destination.
#[derive(Debug, Clone)]
pub struct ServerAlertConfig {
    pub server_id: uuid::Uuid,
    pub webhook_url: String,
    pub settings: AlertSettings,
}

#[async_trait]
pub trait GameAlertRepository: Send + Sync {
    /// Reglages d'un serveur, s'il en a.
    async fn find(&self, server_id: uuid::Uuid) -> Result<Option<ServerAlertConfig>, DomainError>;

    /// Cree ou remplace les reglages. Les dates de dernier envoi sont
    /// CONSERVEES : changer un seuil ne doit pas rouvrir la porte a une salve
    /// d'alertes deja envoyees.
    async fn upsert(
        &self,
        server_id: uuid::Uuid,
        webhook_url: &str,
        cpu_threshold: i32,
        ram_threshold: i32,
        latency_threshold_ms: i32,
        actor: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Retire les alertes d'un serveur.
    async fn delete(&self, server_id: uuid::Uuid) -> Result<bool, DomainError>;

    /// Marque une alerte comme envoyee, pour que le delai coure.
    async fn mark_sent(&self, server_id: uuid::Uuid, kind: AlertKind) -> Result<(), DomainError>;
}
