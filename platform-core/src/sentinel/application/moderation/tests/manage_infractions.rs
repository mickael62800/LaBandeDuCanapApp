use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::sentinel::application::moderation::manage_infractions_service::ManageInfractionsService;
use crate::sentinel::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::sentinel::domain::entities::moderation::infraction::Infraction;
use crate::sentinel::domain::enums::moderation::action::Action;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::manage_infractions::InfractionFilters;
use crate::sentinel::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use crate::sentinel::ports::outbound::moderation::infraction_repository::InfractionRepository;

fn sample(id: &str) -> Infraction {
    Infraction {
        id: Uuid::parse_str(id).unwrap_or_else(|_| Uuid::new_v4()),
        guild_id: "g".into(),
        channel_id: "c".into(),
        user_id: "u".into(),
        username: "u".into(),
        display_name: None,
        message_id: "m".into(),
        content: "".into(),
        flags: DetectionFlags {
            spam: false,
            insult: false,
            profanity: false,
            link: false,
            phishing: false,
        },
        score: 0.0,
        action: Action::Warn,
        reason: "".into(),
        duration: None,
        created_at: Utc::now(),
    }
}

#[derive(Default)]
struct MockRepo {
    list_calls: Mutex<Vec<(String, i64, i64)>>,
    all_calls: Mutex<Vec<(i64, i64)>>,
    deletes: Mutex<Vec<String>>,
    delete_older: Mutex<Vec<(String, i32)>>,
    find_by_id_returns: Mutex<Option<Infraction>>,
    delete_returns: Mutex<bool>,
    infractions: Mutex<Vec<Infraction>>,
    /// Reponse brute de l'agregation SQL (`GROUP BY action`).
    counts_by_action: Mutex<Vec<(String, u64)>>,
}

#[async_trait]
impl InfractionRepository for MockRepo {
    async fn count_by_action_for_user(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Vec<(String, u64)>, crate::sentinel::domain::errors::DomainError> {
        Ok(self.counts_by_action.lock().unwrap().clone())
    }
    async fn save(&self, _: &Infraction) -> Result<(), DomainError> {
        Ok(())
    }
    async fn find_by_guild(
        &self,
        g: &str,
        f: &InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError> {
        self.list_calls
            .lock()
            .unwrap()
            .push((g.into(), f.limit, f.offset));
        Ok(self.infractions.lock().unwrap().clone())
    }
    async fn find_all(&self, limit: i64, offset: i64) -> Result<Vec<Infraction>, DomainError> {
        self.all_calls.lock().unwrap().push((limit, offset));
        Ok(self.infractions.lock().unwrap().clone())
    }
    async fn count_today(&self) -> Result<u64, DomainError> {
        Ok(13)
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<Infraction>, DomainError> {
        Ok(self.find_by_id_returns.lock().unwrap().clone())
    }
    async fn delete_by_id(&self, id: &str) -> Result<bool, DomainError> {
        self.deletes.lock().unwrap().push(id.into());
        Ok(*self.delete_returns.lock().unwrap())
    }
    async fn delete_older_than_days(&self, g: &str, d: i32) -> Result<u64, DomainError> {
        self.delete_older.lock().unwrap().push((g.into(), d));
        Ok(100)
    }
}

#[tokio::test]
async fn list_infractions_forwards_filters() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageInfractionsService::new(r.clone());
    let f = InfractionFilters {
        user_id: None,
        action: None,
        limit: 25,
        offset: 5,
    };
    svc.list_infractions("g1", f).await.unwrap();
    assert_eq!(r.list_calls.lock().unwrap()[0], ("g1".into(), 25, 5));
}

#[tokio::test]
async fn list_all_infractions_forwards_limit_offset() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageInfractionsService::new(r.clone());
    svc.list_all_infractions(100, 0).await.unwrap();
    assert_eq!(r.all_calls.lock().unwrap()[0], (100, 0));
}

#[tokio::test]
async fn count_today_forwards() {
    let svc = ManageInfractionsService::new(Arc::new(MockRepo::default()));
    assert_eq!(svc.count_today().await.unwrap(), 13);
}

#[tokio::test]
async fn find_by_id_returns_some_or_none() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageInfractionsService::new(r.clone());
    assert!(svc.find_by_id("x").await.unwrap().is_none());
    *r.find_by_id_returns.lock().unwrap() = Some(sample("00000000-0000-0000-0000-000000000001"));
    assert!(svc.find_by_id("x").await.unwrap().is_some());
}

#[tokio::test]
async fn delete_infraction_forwards_bool_result() {
    let r = Arc::new(MockRepo::default());
    *r.delete_returns.lock().unwrap() = true;
    let svc = ManageInfractionsService::new(r.clone());
    assert!(svc.delete_infraction("abc").await.unwrap());
    assert_eq!(r.deletes.lock().unwrap()[0], "abc");
}

#[tokio::test]
async fn delete_older_than_days_forwards() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageInfractionsService::new(r.clone());
    let n = svc.delete_older_than_days("g", 90).await.unwrap();
    assert_eq!(n, 100);
    assert_eq!(r.delete_older.lock().unwrap()[0], ("g".into(), 90));
}

// ── Compteurs d'infractions ──

fn service_avec_counts(rows: Vec<(String, u64)>) -> ManageInfractionsService {
    let repo = Arc::new(MockRepo::default());
    *repo.counts_by_action.lock().unwrap() = rows;
    ManageInfractionsService::new(repo)
}

#[tokio::test]
async fn compte_les_quatre_natures_detaillees() {
    let svc = service_avec_counts(vec![
        ("warn".into(), 3),
        ("delete".into(), 2),
        ("mute".into(), 1),
        ("ban".into(), 5),
    ]);
    let c = svc.count_user_infractions("g", "u").await.unwrap();
    assert_eq!((c.warns, c.deletes, c.mutes, c.bans), (3, 2, 1, 5));
    assert_eq!(c.total, 11);
}

#[tokio::test]
async fn une_nature_inconnue_alimente_le_total_seulement() {
    // Le journal peut porter des natures sans compteur dedie (kick, purge...).
    // Elles ne doivent pas disparaitre du total affiche.
    let svc = service_avec_counts(vec![("warn".into(), 1), ("kick".into(), 4)]);
    let c = svc.count_user_infractions("g", "u").await.unwrap();
    assert_eq!(c.warns, 1);
    assert_eq!((c.deletes, c.mutes, c.bans), (0, 0, 0));
    assert_eq!(c.total, 5);
}

#[tokio::test]
async fn aucun_resultat_donne_des_compteurs_a_zero() {
    let svc = service_avec_counts(vec![]);
    let c = svc.count_user_infractions("g", "u").await.unwrap();
    assert_eq!(c, Default::default());
}
