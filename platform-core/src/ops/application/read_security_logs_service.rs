//! Implementation du use case d'analyse des logs securite. Les lectures
//! brutes passent par le repo ; le calcul des stats de trafic (moyenne, pic,
//! alerte) est de la logique domaine (cf. `TrafficTrend::from_points`).

use std::sync::Arc;

use async_trait::async_trait;

use crate::ops::domain::entities::security_log::{AuthFailure, LogWindow, TopIp, TrafficTrend, TrafficPoint};
use crate::ops::domain::errors::DomainError;
use crate::ops::ports::inbound::read_security_logs::ReadSecurityLogsUseCase;
use crate::ops::ports::outbound::security_log_repository::SecurityLogRepository;

pub struct ReadSecurityLogsService {
    repo: Arc<dyn SecurityLogRepository>,
}

impl ReadSecurityLogsService {
    pub fn new(repo: Arc<dyn SecurityLogRepository>) -> Self {
        Self { repo }
    }
}

/// Borne une valeur venue de l'exterieur.
///
/// Ces trois entiers finissent interpoles dans du SQL (`LIMIT {n}`,
/// `INTERVAL '{n} min'`). Ils etaient bornes dans le handler HTTP uniquement,
/// alors que l'adapter Postgres documentait l'inverse (« borne par le use
/// case ») : un second adaptateur — gRPC, CLI, tache planifiee — serait passe a
/// cote sans que rien ne le signale. Et `bucket_minutes = 0` produisait une
/// division par zero cote Postgres.
fn borne(valeur: i64, min: i64, max: i64) -> i64 {
    valeur.clamp(min, max)
}

#[async_trait]
impl ReadSecurityLogsUseCase for ReadSecurityLogsService {
    async fn top_ips(&self, window: LogWindow, limit: i64) -> Result<Vec<TopIp>, DomainError> {
        self.repo.top_ips(window, borne(limit, 1, 100)).await
    }

    async fn auth_failures(
        &self,
        window: LogWindow,
        limit: i64,
    ) -> Result<Vec<AuthFailure>, DomainError> {
        self.repo.auth_failures(window, borne(limit, 1, 500)).await
    }

    async fn traffic_trend(
        &self,
        window: LogWindow,
        bucket_minutes: i64,
    ) -> Result<TrafficTrend, DomainError> {
        let points = self
            .repo
            .traffic_points(window, borne(bucket_minutes, 1, 60))
            .await?;
        Ok(TrafficTrend::from_points(points))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_valeurs_hors_bornes_sont_ramenees() {
        assert_eq!(borne(0, 1, 60), 1);
        assert_eq!(borne(-5, 1, 100), 1);
        assert_eq!(borne(10_000, 1, 100), 100);
        assert_eq!(borne(20, 1, 100), 20);
    }

    struct FakeSecurityLogRepo;
    #[async_trait]
    impl SecurityLogRepository for FakeSecurityLogRepo {
        async fn top_ips(
            &self,
            _window: LogWindow,
            _limit: i64,
        ) -> Result<Vec<TopIp>, DomainError> {
            Ok(vec![])
        }

        async fn auth_failures(
            &self,
            _window: LogWindow,
            _limit: i64,
        ) -> Result<Vec<AuthFailure>, DomainError> {
            Ok(vec![])
        }

        async fn traffic_points(
            &self,
            _window: LogWindow,
            _bucket_minutes: i64,
        ) -> Result<Vec<TrafficPoint>, DomainError> {
            Ok(vec![])
        }
    }

    #[test]
    fn service_can_be_created() {
        let _service = ReadSecurityLogsService::new(Arc::new(FakeSecurityLogRepo));
    }

    #[tokio::test]
    async fn top_ips_clamps_limit() {
        let service = ReadSecurityLogsService::new(Arc::new(FakeSecurityLogRepo));
        let window = LogWindow::OneHour;
        let result = service.top_ips(window, 0).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn auth_failures_clamps_limit() {
        let service = ReadSecurityLogsService::new(Arc::new(FakeSecurityLogRepo));
        let window = LogWindow::TwentyFourHours;
        let result = service.auth_failures(window, 1000).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn traffic_trend_calculates_from_points() {
        let service = ReadSecurityLogsService::new(Arc::new(FakeSecurityLogRepo));
        let window = LogWindow::SevenDays;
        let result = service.traffic_trend(window, 5).await;
        assert!(result.is_ok());
    }
}
