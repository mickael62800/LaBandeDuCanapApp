use async_trait::async_trait;
use std::sync::{Arc, Mutex};

use crate::nexus::application::coussin_insurance_service::CoussinInsuranceService;
use crate::nexus::application::economy_config::EmptyBotConfigRepository;
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::inbound::coussin_insurance::CoussinInsuranceUseCase;
use crate::nexus::ports::outbound::coussin_insurance_repository::{
    CoussinInsurance, CoussinInsuranceRepository,
};

#[derive(Default)]
struct MockInsuranceRepo {
    insurance: Mutex<Option<CoussinInsurance>>,
}

#[async_trait]
impl CoussinInsuranceRepository for MockInsuranceRepo {
    async fn active(&self, _g: &str, _u: &str) -> Result<Option<CoussinInsurance>, DomainError> {
        Ok(self.insurance.lock().unwrap().clone())
    }
    async fn buy(
        &self,
        _guild_id: &str,
        _user_id: &str,
        is_scam: bool,
        _cost: i64,
        duration_minutes: i64,
    ) -> Result<CoussinInsurance, DomainError> {
        let ins = CoussinInsurance {
            is_scam,
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(duration_minutes),
        };
        *self.insurance.lock().unwrap() = Some(ins.clone());
        Ok(ins)
    }
}

#[tokio::test]
async fn test_buy_insurance_success() {
    let repo = Arc::new(MockInsuranceRepo::default());
    let service = CoussinInsuranceService::new(repo.clone(), Arc::new(EmptyBotConfigRepository));

    let ins = service.buy("g1", "u1").await.unwrap();
    let active = service.active("g1", "u1").await.unwrap();
    assert!(active.is_some());
    assert_eq!(active.unwrap().is_scam, ins.is_scam);
}
