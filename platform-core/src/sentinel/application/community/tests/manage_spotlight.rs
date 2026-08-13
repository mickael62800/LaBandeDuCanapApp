use super::*;
use std::sync::Mutex;

#[derive(Default)]
struct MockRepo {
    upserted: Mutex<Vec<UpsertSpotlightCommand>>,
    latest: Mutex<Option<Spotlight>>,
    par_periode: Mutex<Option<Spotlight>>,
}

#[async_trait]
impl SpotlightRepository for MockRepo {
    async fn find_by_period(
        &self,
        _guild_id: &str,
        _period: &str,
    ) -> Result<Option<Spotlight>, DomainError> {
        Ok(self.par_periode.lock().unwrap().clone())
    }

    async fn find_latest(&self, _guild_id: &str) -> Result<Option<Spotlight>, DomainError> {
        Ok(self.latest.lock().unwrap().clone())
    }

    async fn list(&self, _guild_id: &str, _limit: i64) -> Result<Vec<Spotlight>, DomainError> {
        Ok(vec![])
    }

    async fn upsert(&self, cmd: &UpsertSpotlightCommand) -> Result<Spotlight, DomainError> {
        self.upserted.lock().unwrap().push(cmd.clone());
        Ok(Spotlight {
            id: Uuid::nil(),
            guild_id: cmd.guild_id.clone(),
            user_id: cmd.user_id.clone(),
            username: cmd.username.clone(),
            avatar: cmd.avatar.clone(),
            period: cmd.period.clone().unwrap_or_default(),
            reason: cmd.reason.clone(),
            chosen_by: cmd.chosen_by.clone(),
            created_at: Utc::now(),
        })
    }

    async fn delete(&self, _id: Uuid) -> Result<bool, DomainError> {
        Ok(false)
    }
}

fn cmd() -> UpsertSpotlightCommand {
    UpsertSpotlightCommand {
        guild_id: "g".into(),
        user_id: "tiya".into(),
        username: "  Tiya  ".into(),
        avatar: Some("   ".into()),
        period: None,
        reason: "  Accueille les nouveaux.  ".into(),
        chosen_by: "staff".into(),
    }
}

fn spot(period: &str) -> Spotlight {
    Spotlight {
        id: Uuid::nil(),
        guild_id: "g".into(),
        user_id: "tiya".into(),
        username: "Tiya".into(),
        avatar: None,
        period: period.into(),
        reason: "Accueille les nouveaux.".into(),
        chosen_by: "staff".into(),
        created_at: Utc::now(),
    }
}

fn service(repo: MockRepo) -> (ManageSpotlightService, Arc<MockRepo>) {
    let repo = Arc::new(repo);
    (ManageSpotlightService::new(repo.clone()), repo)
}

#[tokio::test]
async fn designation_rogne_les_champs() {
    let (svc, repo) = service(MockRepo::default());
    svc.designate(cmd()).await.unwrap();

    let saved = repo.upserted.lock().unwrap()[0].clone();
    assert_eq!(saved.username, "Tiya");
    assert_eq!(saved.reason, "Accueille les nouveaux.");
    assert!(saved.avatar.is_none(), "avatar blanc => absent");
}

/// La raison est ce qui donne son sens a la distinction : sans elle, la
/// section n'affiche qu'un nom.
#[tokio::test]
async fn raison_vide_est_refusee() {
    let (svc, _) = service(MockRepo::default());
    let mut c = cmd();
    c.reason = "   ".into();
    assert!(matches!(
        svc.designate(c).await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn membre_absent_est_refuse() {
    let (svc, _) = service(MockRepo::default());
    let mut c = cmd();
    c.user_id = "  ".into();
    assert!(matches!(
        svc.designate(c).await,
        Err(DomainError::ValidationError(_))
    ));
}

/// Cas de loin le plus frequent : le staff designe pour le mois en cours.
#[tokio::test]
async fn periode_absente_prend_le_mois_courant() {
    let (svc, repo) = service(MockRepo::default());
    svc.designate(cmd()).await.unwrap();

    let saved = repo.upserted.lock().unwrap()[0].clone();
    assert_eq!(
        saved.period.as_deref(),
        Some(period_of(Utc::now()).as_str())
    );
}

#[tokio::test]
async fn periode_explicite_est_conservee() {
    let (svc, repo) = service(MockRepo::default());
    let mut c = cmd();
    c.period = Some("2026-03".into());
    svc.designate(c).await.unwrap();

    assert_eq!(
        repo.upserted.lock().unwrap()[0].period.as_deref(),
        Some("2026-03")
    );
}

#[tokio::test]
async fn periode_mal_formee_est_refusee_avant_la_base() {
    let (svc, _) = service(MockRepo::default());
    let mut c = cmd();
    c.period = Some("mars 2026".into());
    assert!(matches!(
        svc.designate(c).await,
        Err(DomainError::ValidationError(_))
    ));
}

/// Sans periode demandee, on prend le plus recent : tant que le staff n'a
/// designe personne pour ce mois-ci, la section continue de montrer le
/// precedent au lieu de disparaitre.
#[tokio::test]
async fn consultation_sans_periode_retombe_sur_le_plus_recent() {
    let repo = MockRepo {
        latest: Mutex::new(Some(spot("2026-01"))),
        ..Default::default()
    };
    let (svc, _) = service(repo);

    let vu = svc.current("g", None).await.unwrap().unwrap();
    assert_eq!(vu.period, "2026-01");
}

#[tokio::test]
async fn consultation_d_une_periode_precise_l_interroge() {
    let repo = MockRepo {
        par_periode: Mutex::new(Some(spot("2025-12"))),
        latest: Mutex::new(Some(spot("2026-01"))),
        ..Default::default()
    };
    let (svc, _) = service(repo);

    let vu = svc.current("g", Some("2025-12")).await.unwrap().unwrap();
    assert_eq!(vu.period, "2025-12");
}

#[tokio::test]
async fn consultation_d_une_periode_mal_formee_est_refusee() {
    let (svc, _) = service(MockRepo::default());
    assert!(matches!(
        svc.current("g", Some("2026-13")).await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn aucune_designation_ne_remonte_rien_sans_erreur() {
    let (svc, _) = service(MockRepo::default());
    assert!(svc.current("g", None).await.unwrap().is_none());
}
