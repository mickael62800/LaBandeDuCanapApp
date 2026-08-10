use async_trait::async_trait;
use std::sync::{Arc, Mutex};

use crate::application::coussin_bet_service::CoussinBetService;
use crate::application::economy_config::EmptyBotConfigRepository;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coussin_bet::CoussinBetUseCase;
use crate::ports::outbound::coussin_bet_repository::CoussinBetRepository;
use crate::ports::outbound::coussin_cooldown_repository::CoussinCooldownRepository;

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
struct MockBetRepo {
    placed: Mutex<Option<(uuid::Uuid, String, String, i64)>>,
}

#[async_trait]
impl CoussinBetRepository for MockBetRepo {
    async fn place(
        &self,
        _guild_id: &str,
        combat_id: uuid::Uuid,
        bettor_id: &str,
        _bettor_name: &str,
        backed_id: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        *self.placed.lock().unwrap() =
            Some((combat_id, bettor_id.into(), backed_id.into(), amount));
        Ok(())
    }
}

#[tokio::test]
async fn test_cannot_place_bet_below_min() {
    let service = CoussinBetService::new(
        Arc::new(MockBetRepo::default()),
        Arc::new(EmptyBotConfigRepository),
        Arc::new(MockCooldownRepo),
    );
    // Min is 10 by default
    let res = service
        .place("g1", uuid::Uuid::new_v4(), "b1", "Bettor", "p1", 5)
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_place_bet_success() {
    let repo = Arc::new(MockBetRepo::default());
    let service = CoussinBetService::new(
        repo.clone(),
        Arc::new(EmptyBotConfigRepository),
        Arc::new(MockCooldownRepo),
    );
    let combat_id = uuid::Uuid::new_v4();
    let res = service
        .place("g1", combat_id, "b1", "Bettor", "p1", 50)
        .await;
    assert!(res.is_ok());

    let placed = repo.placed.lock().unwrap();
    assert_eq!(*placed, Some((combat_id, "b1".into(), "p1".into(), 50)));
}
