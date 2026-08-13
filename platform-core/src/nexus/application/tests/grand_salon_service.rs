use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{Duration, TimeZone, Utc};
use uuid::Uuid;

use super::*;
use crate::nexus::domain::entities::grand_salon::{
    Cercle, Dossier, GazetteArticle, Habitué, MotionDuSalon, MotionStatus, Ressources,
};
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::outbound::grand_salon_repository::GrandSalonRepository;

#[derive(Default)]
struct FakeState {
    habitue: Option<Habitué>,
    daily_claimed: bool,
    motions: Vec<MotionDuSalon>,
    votes: Vec<(Uuid, Uuid, bool, i64)>,
    closed: Vec<(Uuid, bool)>,
    articles: Vec<GazetteArticle>,
}

#[derive(Default)]
struct FakeRepo(Mutex<FakeState>);

#[async_trait]
impl GrandSalonRepository for FakeRepo {
    async fn find_habitue(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Habitué>, DomainError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .habitue
            .clone()
            .filter(|h| h.guild_id == guild_id && h.user_id == user_id))
    }
    async fn save_habitue(&self, habitue: &Habitué) -> Result<(), DomainError> {
        self.0.lock().unwrap().habitue = Some(habitue.clone());
        Ok(())
    }
    async fn claim_daily(&self, _habitue_id: Uuid) -> Result<bool, DomainError> {
        let mut s = self.0.lock().unwrap();
        if s.daily_claimed {
            return Ok(false);
        }
        s.daily_claimed = true;
        if let Some(h) = &mut s.habitue {
            h.ressources.rayonnement += 10;
            h.ressources.jetons += 50;
            h.ressources.reputation += 2;
            h.ressources.bons_plans += 3;
            h.ressources.reseau += 2;
        }
        Ok(true)
    }
    async fn create_cercle(&self, _cercle: &Cercle) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_cercles(&self, _guild_id: &str) -> Result<Vec<Cercle>, DomainError> {
        Ok(vec![])
    }
    async fn create_motion(&self, motion: &MotionDuSalon) -> Result<(), DomainError> {
        self.0.lock().unwrap().motions.push(motion.clone());
        Ok(())
    }
    async fn list_motions(&self, guild_id: &str) -> Result<Vec<MotionDuSalon>, DomainError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .motions
            .iter()
            .filter(|m| m.guild_id == guild_id)
            .cloned()
            .collect())
    }
    async fn cast_vote(
        &self,
        motion_id: Uuid,
        habitue_id: Uuid,
        choice: bool,
        weight: i64,
    ) -> Result<(), DomainError> {
        self.0
            .lock()
            .unwrap()
            .votes
            .push((motion_id, habitue_id, choice, weight));
        Ok(())
    }
    async fn vote_totals(&self, motion_id: Uuid) -> Result<(i64, i64), DomainError> {
        let s = self.0.lock().unwrap();
        Ok(s.votes
            .iter()
            .filter(|v| v.0 == motion_id)
            .fold(
                (0, 0),
                |(p, c), v| if v.2 { (p + v.3, c) } else { (p, c + v.3) },
            ))
    }
    async fn due_motions(&self) -> Result<Vec<MotionDuSalon>, DomainError> {
        Ok(self.0.lock().unwrap().motions.clone())
    }
    async fn close_motion(&self, id: Uuid, adopted: bool) -> Result<(), DomainError> {
        self.0.lock().unwrap().closed.push((id, adopted));
        Ok(())
    }
    async fn publish_gazette(&self, article: &GazetteArticle) -> Result<(), DomainError> {
        self.0.lock().unwrap().articles.push(article.clone());
        Ok(())
    }
    async fn list_gazette(&self, _guild_id: &str) -> Result<Vec<GazetteArticle>, DomainError> {
        Ok(self.0.lock().unwrap().articles.clone())
    }
    async fn create_dossier(&self, _dossier: &Dossier) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_dossiers(
        &self,
        _guild_id: &str,
        _owner_id: Uuid,
    ) -> Result<Vec<Dossier>, DomainError> {
        Ok(vec![])
    }
    async fn reveal_dossier(&self, _dossier_id: Uuid, _owner_id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
}

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 10, 12, 0, 0).unwrap()
}
fn member(rayonnement: i64) -> Habitué {
    Habitué {
        id: Uuid::nil(),
        guild_id: "guild".into(),
        user_id: "user".into(),
        display_name: "Lina".into(),
        ressources: Ressources {
            rayonnement,
            jetons: 1000,
            reputation: 0,
            bons_plans: 0,
            reseau: 0,
        },
        joined_at: now(),
    }
}
fn motion() -> MotionDuSalon {
    MotionDuSalon {
        id: Uuid::new_v4(),
        guild_id: "guild".into(),
        titre: "Soirée plaid".into(),
        texte: "On sort les plaids.".into(),
        status: MotionStatus::EnVote,
        author_id: Uuid::nil(),
        closes_at: Utc::now() + Duration::hours(1),
        soutien_pour: 0,
        soutien_contre: 0,
    }
}

#[tokio::test]
async fn join_is_idempotent_and_keeps_existing_resources() {
    let repo = Arc::new(FakeRepo::default());
    let service = GrandSalonService::new(repo.clone(), 1000);
    let first = service.join("guild", "user", "Lina", now()).await.unwrap();
    repo.0
        .lock()
        .unwrap()
        .habitue
        .as_mut()
        .unwrap()
        .ressources
        .jetons = 1234;
    let second = service
        .join("guild", "user", "Nouveau nom", now())
        .await
        .unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(second.ressources.jetons, 1234);
}

#[tokio::test]
async fn daily_rewards_once_only() {
    let repo = Arc::new(FakeRepo::default());
    repo.0.lock().unwrap().habitue = Some(member(0));
    let service = GrandSalonService::new(repo, 1000);
    let rewarded = service.daily("guild", "user").await.unwrap();
    assert_eq!(rewarded.ressources.rayonnement, 10);
    assert_eq!(rewarded.ressources.jetons, 1050);
    assert!(service.daily("guild", "user").await.is_err());
}

#[tokio::test]
async fn vote_weight_is_capped_by_rayonnement() {
    let repo = Arc::new(FakeRepo::default());
    repo.0.lock().unwrap().habitue = Some(member(10_000));
    repo.0.lock().unwrap().motions.push(motion());
    let id = repo.0.lock().unwrap().motions[0].id;
    let service = GrandSalonService::new(repo.clone(), 1000);
    service.vote("guild", "user", id, true).await.unwrap();
    assert_eq!(repo.0.lock().unwrap().votes[0].3, 5);
}

#[tokio::test]
async fn invalid_motion_is_not_persisted() {
    let repo = Arc::new(FakeRepo::default());
    let service = GrandSalonService::new(repo.clone(), 1000);
    let mut invalid = motion();
    invalid.titre = " ".into();
    assert!(service.propose_motion(invalid).await.is_err());
    assert!(repo.0.lock().unwrap().motions.is_empty());
}

#[tokio::test]
async fn closing_due_motion_uses_votes_and_publishes_gazette() {
    let repo = Arc::new(FakeRepo::default());
    let m = motion();
    let id = m.id;
    repo.0.lock().unwrap().motions.push(m);
    repo.0
        .lock()
        .unwrap()
        .votes
        .push((id, Uuid::nil(), true, 2));
    let service = GrandSalonService::new(repo.clone(), 1000);
    assert_eq!(service.close_due_motions(&[], now()).await.unwrap(), 1);
    let state = repo.0.lock().unwrap();
    assert_eq!(state.closed, vec![(id, true)]);
    assert_eq!(state.articles.len(), 1);
    assert!(state.articles[0].headline.contains("adoptée"));
}
