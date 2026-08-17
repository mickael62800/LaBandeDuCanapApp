use async_trait::async_trait;
use std::sync::{Arc, Mutex};

use crate::nexus::application::coussin_service::CoussinService;
use crate::nexus::application::economy_config::EnabledBotConfigRepository;
use crate::nexus::domain::entities::coussin::PlayerClass;
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::inbound::coussin_profile::{CoussinCombatUseCase, CoussinProfileUseCase};
use crate::nexus::ports::outbound::coussin_cooldown_repository::CoussinCooldownRepository;
use crate::nexus::ports::outbound::coussin_repository::*;

#[derive(Default)]
struct MockCooldownRepo {
    remaining: Mutex<Option<i64>>,
}

#[async_trait]
impl CoussinCooldownRepository for MockCooldownRepo {
    async fn remaining_seconds(
        &self,
        _g: &str,
        _u: &str,
        _a: &str,
    ) -> Result<Option<i64>, DomainError> {
        Ok(*self.remaining.lock().unwrap())
    }
    async fn arm(&self, _g: &str, _u: &str, _a: &str, _m: i64) -> Result<(), DomainError> {
        Ok(())
    }
}

#[derive(Default)]
struct MockCoussinRepo {
    profiles: Mutex<Vec<CoussinProfile>>,
    combats: Mutex<Vec<CoussinCombat>>,
}

#[async_trait]
impl CoussinRepository for MockCoussinRepo {
    async fn list_combat_history(
        &self,
        _g: &str,
        _u: &str,
        _l: i64,
    ) -> Result<Vec<CoussinCombatResult>, DomainError> {
        Ok(vec![])
    }
    async fn list_bets(&self, _g: &str, _u: &str, _l: i64) -> Result<Vec<CoussinBet>, DomainError> {
        Ok(vec![])
    }
    async fn list_primes(
        &self,
        _g: &str,
        _u: &str,
        _l: i64,
    ) -> Result<Vec<CoussinPrime>, DomainError> {
        Ok(vec![])
    }
    async fn find_profile(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoussinProfile>, DomainError> {
        let list = self.profiles.lock().unwrap();
        Ok(list
            .iter()
            .find(|p| p.guild_id == guild_id && p.user_id == user_id)
            .cloned())
    }
    async fn list_profiles(
        &self,
        _guild_id: &str,
        _limit: i64,
    ) -> Result<Vec<CoussinProfile>, DomainError> {
        Ok(self.profiles.lock().unwrap().clone())
    }
    async fn create_profile(&self, profile: &CoussinProfile) -> Result<(), DomainError> {
        self.profiles.lock().unwrap().push(profile.clone());
        Ok(())
    }
    async fn update_class(
        &self,
        guild_id: &str,
        user_id: &str,
        class: PlayerClass,
        atk: i32,
        def: i32,
        hp_max: i32,
        _cooldown_minutes: i64,
    ) -> Result<(), DomainError> {
        let mut list = self.profiles.lock().unwrap();
        if let Some(p) = list
            .iter_mut()
            .find(|p| p.guild_id == guild_id && p.user_id == user_id)
        {
            p.class = class;
            p.atk = atk;
            p.def = def;
            p.hp_max = hp_max;
        }
        Ok(())
    }
    async fn spend_stat_point(
        &self,
        guild_id: &str,
        user_id: &str,
        stat: &str,
    ) -> Result<CoussinProfile, DomainError> {
        let mut list = self.profiles.lock().unwrap();
        let p = list
            .iter_mut()
            .find(|p| p.guild_id == guild_id && p.user_id == user_id)
            .unwrap();
        p.stat_points -= 1;
        match stat {
            "atk" => p.atk += 5,
            "def" => p.def += 5,
            "hp" => p.hp_max += 20,
            _ => {}
        }
        Ok(p.clone())
    }
    async fn create_combat(
        &self,
        guild_id: &str,
        _channel_id: &str,
        attacker: &CoussinProfile,
        defender: &CoussinProfile,
        mise: i64,
        _cooldown_minutes: i64,
    ) -> Result<CoussinCombat, DomainError> {
        let c = CoussinCombat {
            id: uuid::Uuid::new_v4(),
            guild_id: guild_id.into(),
            attacker_id: attacker.user_id.clone(),
            defender_id: defender.user_id.clone(),
            mise,
            status: "pending".into(),
        };
        self.combats.lock().unwrap().push(c.clone());
        Ok(c)
    }
    async fn expire_pending_combats(&self) -> Result<Vec<ExpiredCombat>, DomainError> {
        Ok(vec![])
    }
    async fn accept_combat(&self, id: uuid::Uuid, _defender_id: &str) -> Result<bool, DomainError> {
        let mut list = self.combats.lock().unwrap();
        if let Some(c) = list.iter_mut().find(|c| c.id == id) {
            c.status = "accepted".into();
            Ok(true)
        } else {
            Ok(false)
        }
    }
    async fn refuse_combat(&self, id: uuid::Uuid, _defender_id: &str) -> Result<bool, DomainError> {
        let mut list = self.combats.lock().unwrap();
        if let Some(c) = list.iter_mut().find(|c| c.id == id) {
            c.status = "refused".into();
            Ok(true)
        } else {
            Ok(false)
        }
    }
    async fn resolution_snapshot(
        &self,
        _id: uuid::Uuid,
    ) -> Result<Option<CoussinCombatSnapshot>, DomainError> {
        Ok(None)
    }
    async fn resolve_combat(
        &self,
        _id: uuid::Uuid,
        _winner_id: Option<&str>,
        _attacker_roll: i32,
        _defender_roll: i32,
        _transferred: i64,
        _attacker_hp: i32,
        _defender_hp: i32,
        _bet_payout_pct: i64,
        _attacker_progress: Option<CoussinProgress>,
        _defender_progress: Option<CoussinProgress>,
    ) -> Result<bool, DomainError> {
        Ok(true)
    }
}

fn sample_profile(guild_id: &str, user_id: &str, username: &str) -> CoussinProfile {
    CoussinProfile {
        guild_id: guild_id.into(),
        user_id: user_id.into(),
        username: username.into(),
        class: PlayerClass::Couette,
        level: 1,
        xp: 0,
        atk: 20,
        def: 20,
        hp_current: 100,
        hp_max: 100,
        coins: 500,
        stat_points: 2,
        title: "Debutant".into(),
        total_wins: 0,
        total_losses: 0,
        total_draws: 0,
        total_stolen: 0,
        cowardice_count: 0,
        chaos_events: 0,
    }
}

#[tokio::test]
async fn test_choose_class_success() {
    let repo = Arc::new(MockCoussinRepo::default());
    repo.create_profile(&sample_profile("g1", "u1", "Player1"))
        .await
        .unwrap();

    let service = CoussinService::new(
        repo.clone(),
        Arc::new(EnabledBotConfigRepository),
        Arc::new(MockCooldownRepo::default()),
    );

    let updated = service
        .choose_class("g1", "u1", "Player1", "ecraseur")
        .await
        .unwrap();
    assert_eq!(updated.class, PlayerClass::Ecraseur);
}

#[tokio::test]
async fn test_train_stat_point() {
    let repo = Arc::new(MockCoussinRepo::default());
    repo.create_profile(&sample_profile("g1", "u1", "Player1"))
        .await
        .unwrap();

    let service = CoussinService::new(
        repo.clone(),
        Arc::new(EnabledBotConfigRepository),
        Arc::new(MockCooldownRepo::default()),
    );

    let updated = service.train("g1", "u1", "Player1", "atk").await.unwrap();
    assert_eq!(updated.stat_points, 1);
    assert_eq!(updated.atk, 25);
}

#[tokio::test]
async fn test_challenge_creation() {
    let repo = Arc::new(MockCoussinRepo::default());
    repo.create_profile(&sample_profile("g1", "u1", "Attacker"))
        .await
        .unwrap();
    repo.create_profile(&sample_profile("g1", "u2", "Defender"))
        .await
        .unwrap();

    let service = CoussinService::new(
        repo.clone(),
        Arc::new(EnabledBotConfigRepository),
        Arc::new(MockCooldownRepo::default()),
    );

    let combat = service
        .challenge("g1", "chan1", "u1", "Attacker", "u2", "Defender", 50)
        .await
        .unwrap();
    assert_eq!(combat.attacker_id, "u1");
    assert_eq!(combat.defender_id, "u2");
    assert_eq!(combat.mise, 50);
}
