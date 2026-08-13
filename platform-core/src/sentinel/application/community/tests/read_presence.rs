use super::*;
use chrono::Duration;
use std::sync::Mutex;

use crate::sentinel::domain::entities::community::presence::{
    VoiceChannelPresence, STALE_AFTER_SECONDS, TEXT_WINDOW_SECONDS,
};

#[derive(Default)]
struct MockRepo {
    voice: Mutex<Option<VoicePresence>>,
    text: Mutex<Vec<TextChannelActivity>>,
    limites_vues: Mutex<Vec<i64>>,
}

#[async_trait]
impl PresenceRepository for MockRepo {
    async fn voice(&self, _: &str) -> Result<Option<VoicePresence>, DomainError> {
        Ok(self.voice.lock().unwrap().clone())
    }

    async fn text_activity(
        &self,
        _: &str,
        limit: i64,
    ) -> Result<Vec<TextChannelActivity>, DomainError> {
        self.limites_vues.lock().unwrap().push(limit);
        Ok(self.text.lock().unwrap().clone())
    }
}

fn presence(age_secondes: i64, salons: usize) -> VoicePresence {
    VoicePresence {
        channels: (0..salons)
            .map(|i| VoiceChannelPresence {
                channel_id: format!("c{i}"),
                channel_name: format!("salon {i}"),
                members: vec![],
                restreint: false,
            })
            .collect(),
        updated_at: Utc::now() - Duration::seconds(age_secondes),
    }
}

fn activite(nom: &str, age_secondes: i64) -> TextChannelActivity {
    TextChannelActivity {
        channel_id: nom.into(),
        channel_name: nom.into(),
        recent_authors: vec!["Kalyx".into()],
        last_message_at: Utc::now() - Duration::seconds(age_secondes),
    }
}

fn service(repo: MockRepo) -> (ReadPresenceService, Arc<MockRepo>) {
    let repo = Arc::new(repo);
    (ReadPresenceService::new(repo.clone()), repo)
}

#[tokio::test]
async fn instantane_frais_est_rendu() {
    let repo = MockRepo {
        voice: Mutex::new(Some(presence(5, 2))),
        ..Default::default()
    };
    let (svc, _) = service(repo);
    assert!(svc.voice("g").await.unwrap().is_some());
}

/// Le seuil vit dans le service : sinon chaque client devrait le
/// reimplementer, et l'un des trois se tromperait.
#[tokio::test]
async fn instantane_perime_est_ecarte() {
    let repo = MockRepo {
        voice: Mutex::new(Some(presence(STALE_AFTER_SECONDS + 10, 2))),
        ..Default::default()
    };
    let (svc, _) = service(repo);
    assert!(svc.voice("g").await.unwrap().is_none());
}

/// Cas normal quand le bot vient de demarrer : pas une erreur.
#[tokio::test]
async fn absence_d_instantane_n_est_pas_une_erreur() {
    let (svc, _) = service(MockRepo::default());
    assert!(svc.voice("g").await.unwrap().is_none());
}

#[tokio::test]
async fn activite_dans_la_fenetre_est_rendue() {
    let repo = MockRepo {
        text: Mutex::new(vec![activite("general", 60)]),
        ..Default::default()
    };
    let (svc, _) = service(repo);
    assert_eq!(svc.text_activity("g", 5).await.unwrap().len(), 1);
}

/// Les horloges de Redis et de l'API peuvent diverger : on refiltre plutot
/// que de faire confiance a la borne appliquee en amont.
#[tokio::test]
async fn activite_hors_fenetre_est_refiltree() {
    let repo = MockRepo {
        text: Mutex::new(vec![
            activite("vivant", 60),
            activite("mort", TEXT_WINDOW_SECONDS + 60),
        ]),
        ..Default::default()
    };
    let (svc, _) = service(repo);

    let vus = svc.text_activity("g", 5).await.unwrap();
    assert_eq!(vus.len(), 1);
    assert_eq!(vus[0].channel_name, "vivant");
}

#[tokio::test]
async fn limite_est_bornee_avant_d_atteindre_le_depot() {
    let (svc, repo) = service(MockRepo::default());

    svc.text_activity("g", 0).await.unwrap();
    svc.text_activity("g", 10_000).await.unwrap();

    let vues = repo.limites_vues.lock().unwrap().clone();
    assert_eq!(vues[0], 1, "une limite nulle remonte a 1");
    assert!(vues[1] <= 8, "une limite absurde est plafonnee");
}
