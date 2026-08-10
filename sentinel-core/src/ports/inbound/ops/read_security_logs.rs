//! Use case : analyse des logs de securite (top IPs, echecs d'auth, trafic).

use async_trait::async_trait;

use crate::domain::entities::ops::security_log::{AuthFailure, LogWindow, TopIp, TrafficTrend};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ReadSecurityLogsUseCase: Send + Sync {
    async fn top_ips(&self, window: LogWindow, limit: i64) -> Result<Vec<TopIp>, DomainError>;

    async fn auth_failures(
        &self,
        window: LogWindow,
        limit: i64,
    ) -> Result<Vec<AuthFailure>, DomainError>;

    /// Courbe de trafic + stats derivees (moyenne, pic, alerte).
    async fn traffic_trend(
        &self,
        window: LogWindow,
        bucket_minutes: i64,
    ) -> Result<TrafficTrend, DomainError>;
}
