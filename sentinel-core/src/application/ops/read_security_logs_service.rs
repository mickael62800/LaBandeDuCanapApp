//! Implementation du use case d'analyse des logs securite. Les lectures
//! brutes passent par le repo ; le calcul des stats de trafic (moyenne, pic,
//! alerte) est de la logique domaine (cf. `TrafficTrend::from_points`).

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::ops::security_log::{AuthFailure, LogWindow, TopIp, TrafficTrend};
use crate::domain::errors::DomainError;
use crate::ports::inbound::ops::read_security_logs::ReadSecurityLogsUseCase;
use crate::ports::outbound::ops::security_log_repository::SecurityLogRepository;

pub struct ReadSecurityLogsService {
    repo: Arc<dyn SecurityLogRepository>,
}

impl ReadSecurityLogsService {
    pub fn new(repo: Arc<dyn SecurityLogRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ReadSecurityLogsUseCase for ReadSecurityLogsService {
    async fn top_ips(&self, window: LogWindow, limit: i64) -> Result<Vec<TopIp>, DomainError> {
        self.repo.top_ips(window, limit).await
    }

    async fn auth_failures(
        &self,
        window: LogWindow,
        limit: i64,
    ) -> Result<Vec<AuthFailure>, DomainError> {
        self.repo.auth_failures(window, limit).await
    }

    async fn traffic_trend(
        &self,
        window: LogWindow,
        bucket_minutes: i64,
    ) -> Result<TrafficTrend, DomainError> {
        let points = self.repo.traffic_points(window, bucket_minutes).await?;
        Ok(TrafficTrend::from_points(points))
    }
}
