use async_trait::async_trait;
use std::sync::{Arc, Mutex};

use crate::nexus::application::coussin_steal_service::CoussinStealService;
use crate::nexus::application::economy_config::EnabledBotConfigRepository;
use crate::nexus::domain::entities::coussin::PlayerClass;
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::inbound::coussin_steal::CoussinStealUseCase;
use crate::nexus::ports::outbound::coussin_repository::*;
use crate::nexus::ports::outbound::coussin_steal_repository::{
    CoussinStealRepository, StealAttempt,
};

#[derive(Default)]
struct MockStealRepo {
    thief_balance: i64,
    victim_balance: i64,
    transferred: Mutex<Option<(i64, bool)>>,
    /// Tentatives ouvertes, par identifiant.
    attempts: Mutex<Vec<StealAttempt>>,
    outcomes: Mutex<Vec<(uuid::Uuid, bool, bool, i64)>>,
    /// Simule une fenetre deja close (le job est passe avant la victime).
    already_claimed: bool,
}

fn attempt(id: uuid::Uuid) -> StealAttempt {
    StealAttempt {
        id,
        guild_id: "g1".into(),
        thief_id: "voleur".into(),
        victim_id: "victime".into(),
        channel_id: "c1".into(),
        message_id: None,
        expires_at: "2026-08-17T12:00:00Z".into(),
    }
}

#[async_trait]
impl CoussinStealRepository for MockStealRepo {
    async fn balances(&self, _g: &str, _t: &str, _v: &str) -> Result<(i64, i64), DomainError> {
        Ok((self.thief_balance, self.victim_balance))
    }
    async fn settlement_balances(
        &self,
        _g: &str,
        _t: &str,
        _v: &str,
    ) -> Result<(i64, i64), DomainError> {
        Ok((self.thief_balance, self.victim_balance))
    }
    async fn transfer(
        &self,
        _g: &str,
        _t: &str,
        _v: &str,
        amount: i64,
        success: bool,
        _cd: i64,
    ) -> Result<(), DomainError> {
        *self.transferred.lock().unwrap() = Some((amount, success));
        Ok(())
    }
    async fn open_attempt(
        &self,
        _g: &str,
        _t: &str,
        _v: &str,
        _c: &str,
        _window: i64,
    ) -> Result<StealAttempt, DomainError> {
        let created = attempt(uuid::Uuid::new_v4());
        self.attempts.lock().unwrap().push(created.clone());
        Ok(created)
    }
    async fn attach_message(&self, _id: uuid::Uuid, _m: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn claim_attempt(
        &self,
        attempt_id: uuid::Uuid,
        _by_victim: Option<&str>,
    ) -> Result<Option<StealAttempt>, DomainError> {
        if self.already_claimed {
            return Ok(None);
        }
        Ok(Some(attempt(attempt_id)))
    }
    async fn claim_expired_attempts(&self, _l: i64) -> Result<Vec<StealAttempt>, DomainError> {
        Ok(self.attempts.lock().unwrap().clone())
    }
    async fn record_outcome(
        &self,
        attempt_id: uuid::Uuid,
        defended: bool,
        success: bool,
        amount: i64,
    ) -> Result<(), DomainError> {
        self.outcomes
            .lock()
            .unwrap()
            .push((attempt_id, defended, success, amount));
        Ok(())
    }
}

/// Profils : seule la classe du voleur et la DEF de la victime comptent ici.
struct MockProfiles {
    victim_def: i32,
}

fn profile(user_id: &str, class: PlayerClass, def: i32) -> CoussinProfile {
    CoussinProfile {
        guild_id: "g1".into(),
        user_id: user_id.into(),
        username: user_id.into(),
        class,
        level: 1,
        xp: 0,
        atk: 1,
        def,
        hp_current: 100,
        hp_max: 100,
        coins: 500,
        stat_points: 0,
        title: String::new(),
        total_wins: 0,
        total_losses: 0,
        total_draws: 0,
        total_stolen: 0,
        cowardice_count: 0,
        chaos_events: 0,
    }
}

#[async_trait]
impl CoussinRepository for MockProfiles {
    async fn find_profile(
        &self,
        _g: &str,
        user_id: &str,
    ) -> Result<Option<CoussinProfile>, DomainError> {
        Ok(Some(if user_id == "victime" {
            profile("victime", PlayerClass::Couette, self.victim_def)
        } else {
            profile("voleur", PlayerClass::Ecraseur, 10)
        }))
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
    async fn list_combat_history(
        &self,
        _g: &str,
        _u: &str,
        _l: i64,
    ) -> Result<Vec<CoussinCombatResult>, DomainError> {
        Ok(vec![])
    }
    async fn list_profiles(&self, _g: &str, _l: i64) -> Result<Vec<CoussinProfile>, DomainError> {
        Ok(vec![])
    }
    async fn create_profile(&self, _p: &CoussinProfile) -> Result<(), DomainError> {
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    async fn update_class(
        &self,
        _g: &str,
        _u: &str,
        _c: PlayerClass,
        _atk: i32,
        _def: i32,
        _hp_max: i32,
        _cd: i64,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn spend_stat_point(
        &self,
        _g: &str,
        _u: &str,
        _s: &str,
    ) -> Result<CoussinProfile, DomainError> {
        Err(DomainError::NotImplemented("test".into()))
    }
    async fn create_combat(
        &self,
        _g: &str,
        _c: &str,
        _a: &CoussinProfile,
        _d: &CoussinProfile,
        _m: i64,
        _cd: i64,
    ) -> Result<CoussinCombat, DomainError> {
        Err(DomainError::NotImplemented("test".into()))
    }
    async fn accept_combat(&self, _id: uuid::Uuid, _d: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn expire_pending_combats(&self) -> Result<Vec<ExpiredCombat>, DomainError> {
        Ok(vec![])
    }
    async fn refuse_combat(&self, _id: uuid::Uuid, _d: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn resolution_snapshot(
        &self,
        _id: uuid::Uuid,
    ) -> Result<Option<CoussinCombatSnapshot>, DomainError> {
        Ok(None)
    }
    #[allow(clippy::too_many_arguments)]
    async fn resolve_combat(
        &self,
        _id: uuid::Uuid,
        _w: Option<&str>,
        _ar: i32,
        _dr: i32,
        _t: i64,
        _ah: i32,
        _dh: i32,
        _bp: i64,
        _ap: Option<CoussinProgress>,
        _dp: Option<CoussinProgress>,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }
}

fn service(repo: Arc<MockStealRepo>, victim_def: i32) -> CoussinStealService {
    CoussinStealService::new(
        repo,
        Arc::new(MockProfiles { victim_def }),
        Arc::new(EnabledBotConfigRepository),
    )
}

#[tokio::test]
async fn on_ne_se_fouille_pas_soi_meme() {
    let service = service(Arc::new(MockStealRepo::default()), 0);
    assert!(service.open("g1", "u1", "u1", "c1").await.is_err());
}

#[tokio::test]
async fn une_cible_trop_pauvre_est_epargnee() {
    let repo = Arc::new(MockStealRepo {
        thief_balance: 500,
        victim_balance: 5, // sous le plancher par defaut
        ..Default::default()
    });
    assert!(service(repo, 0).open("g1", "u1", "u2", "c1").await.is_err());
}

#[tokio::test]
async fn ouvrir_une_fouille_ne_deplace_aucun_coin() {
    // Rien n'est joue tant que la victime a le temps de reagir : c'est toute
    // la difference avec l'ancien tirage immediat.
    let repo = Arc::new(MockStealRepo {
        thief_balance: 500,
        victim_balance: 500,
        ..Default::default()
    });
    let opened = service(repo.clone(), 0)
        .open("g1", "voleur", "victime", "c1")
        .await
        .expect("fouille ouverte");

    assert!(repo.transferred.lock().unwrap().is_none());
    assert_eq!(opened.defense_window_seconds, 60);
}

#[tokio::test]
async fn se_defendre_a_temps_resout_avec_la_defense_pleine() {
    let repo = Arc::new(MockStealRepo {
        thief_balance: 500,
        victim_balance: 500,
        ..Default::default()
    });
    let outcome = service(repo.clone(), 200)
        .defend(uuid::Uuid::new_v4(), "victime")
        .await
        .expect("resolution");

    assert!(outcome.defended);
    assert_eq!(outcome.absence_malus, 0, "aucun malus quand on reagit");
    // DEF 200 -> +20 : le voleur ne peut pas depasser 20 avec un d20 sans
    // bonus. Reagir a temps rend donc la fouille vaine.
    assert!(!outcome.success);
    assert!(repo.transferred.lock().unwrap().is_some());
}

#[tokio::test]
async fn ne_pas_reagir_coute_le_malus() {
    let repo = Arc::new(MockStealRepo {
        thief_balance: 500,
        victim_balance: 500,
        ..Default::default()
    });
    let svc = service(repo.clone(), 0);
    svc.open("g1", "voleur", "victime", "c1").await.unwrap();

    let outcomes = svc.resolve_expired(10).await.expect("resolution differee");
    assert_eq!(outcomes.len(), 1);
    assert!(!outcomes[0].defended);
    assert_eq!(outcomes[0].absence_malus, 8);
    assert!(repo.transferred.lock().unwrap().is_some());
}

#[tokio::test]
async fn reagir_apres_la_resolution_ne_rejoue_pas_le_vol() {
    // Course normale : la victime clique a la seconde ou le job tranche. Le
    // premier des deux gagne ; le second ne doit RIEN deplacer.
    let repo = Arc::new(MockStealRepo {
        thief_balance: 500,
        victim_balance: 500,
        already_claimed: true,
        ..Default::default()
    });
    let res = service(repo.clone(), 0)
        .defend(uuid::Uuid::new_v4(), "victime")
        .await;

    assert!(res.is_err());
    assert!(repo.transferred.lock().unwrap().is_none());
}
