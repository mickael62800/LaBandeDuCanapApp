//! Port outbound : lectures agregees de la table `logs` (categorie `api`)
//! pour le panel securite.

use async_trait::async_trait;

use crate::domain::entities::ops::security_log::{AuthFailure, LogWindow, TopIp, TrafficPoint};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait SecurityLogRepository: Send + Sync {
    /// Top IPs par nombre de requetes sur la fenetre.
    async fn top_ips(&self, window: LogWindow, limit: i64) -> Result<Vec<TopIp>, DomainError>;

    /// Echecs d'auth (401/403) recents.
    async fn auth_failures(
        &self,
        window: LogWindow,
        limit: i64,
    ) -> Result<Vec<AuthFailure>, DomainError>;

    /// Points de trafic agreges par bucket de `bucket_minutes`.
    async fn traffic_points(
        &self,
        window: LogWindow,
        bucket_minutes: i64,
    ) -> Result<Vec<TrafficPoint>, DomainError>;
}
