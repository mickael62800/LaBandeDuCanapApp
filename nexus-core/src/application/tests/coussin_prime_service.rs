use async_trait::async_trait;
use std::sync::{Arc, Mutex};

use crate::application::coussin_prime_service::CoussinPrimeService;
use crate::application::economy_config::EmptyBotConfigRepository;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coussin_prime::CoussinPrimeUseCase;
use crate::ports::outbound::coussin_cooldown_repository::CoussinCooldownRepository;
use crate::ports::outbound::coussin_prime_repository::CoussinPrimeRepository;

#[derive(Default)]
struct MockCooldownRepo;

#[async_trait]
impl CoussinCooldownRepository for MockCooldownRepo {
    async fn remaining_seconds(
        &self,
        _g: &str,
        _u: &str,
        _a: &str,
    ) -> Result<Option<i64>, DomainError> {
        Ok(None)
    }
    async fn arm(&self, _g: &str, _u: &str, _a: &str, _m: i64) -> Result<(), DomainError> {
        Ok(())
    }
}

#[derive(Default)]
struct MockPrimeRepo {
    placed: Mutex<Option<(String, String, i64)>>,
}

#[async_trait]
impl CoussinPrimeRepository for MockPrimeRepo {
    async fn place(
        &self,
        _guild_id: &str,
        target_id: &str,
        _target_name: &str,
        placer_id: &str,
        _placer_name: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        *self.placed.lock().unwrap() = Some((target_id.into(), placer_id.into(), amount));
        Ok(())
    }
}

#[tokio::test]
async fn test_cannot_place_prime_on_self() {
    let service = CoussinPrimeService::new(
        Arc::new(MockPrimeRepo::default()),
        Arc::new(EmptyBotConfigRepository),
        Arc::new(MockCooldownRepo),
    );
    let res = service
        .place("g1", "u1", "Target", "u1", "Placer", 100)
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_cannot_place_prime_below_min() {
    let service = CoussinPrimeService::new(
        Arc::new(MockPrimeRepo::default()),
        Arc::new(EmptyBotConfigRepository),
        Arc::new(MockCooldownRepo),
    );
    // Min is 10 by default
    let res = service.place("g1", "u2", "Target", "u1", "Placer", 5).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_place_prime_success() {
    let repo = Arc::new(MockPrimeRepo::default());
    let service = CoussinPrimeService::new(
        repo.clone(),
        Arc::new(EmptyBotConfigRepository),
        Arc::new(MockCooldownRepo),
    );
    let res = service
        .place("g1", "u2", "Target", "u1", "Placer", 100)
        .await;
    assert!(res.is_ok());
    let placed = repo.placed.lock().unwrap();
    assert_eq!(*placed, Some(("u2".into(), "u1".into(), 100)));
}
