use super::*;
use chrono::Duration;
use std::sync::Mutex;

use crate::sentinel::domain::entities::community::poll::PollOption;

#[derive(Default)]
struct MockRepo {
    stored: Mutex<Option<Poll>>,
    created: Mutex<Vec<UpsertPollCommand>>,
    votes: Mutex<Vec<(Uuid, String)>>,
    /// Option acceptee par `cast_vote`. Toute autre est rejetee, ce qui
    /// reproduit le controle d'appartenance fait en SQL.
    option_valide: Mutex<Option<Uuid>>,
}

impl MockRepo {
    fn with(poll: Poll, option_valide: Option<Uuid>) -> Self {
        Self {
            stored: Mutex::new(Some(poll)),
            option_valide: Mutex::new(option_valide),
            ..Default::default()
        }
    }
}

#[async_trait]
impl PollRepository for MockRepo {
    async fn list(
        &self,
        _guild_id: &str,
        _open_only: bool,
        _limit: i64,
    ) -> Result<Vec<Poll>, DomainError> {
        Ok(self.stored.lock().unwrap().clone().into_iter().collect())
    }

    async fn find_by_id(&self, _id: Uuid) -> Result<Option<Poll>, DomainError> {
        Ok(self.stored.lock().unwrap().clone())
    }

    async fn create(&self, cmd: &UpsertPollCommand) -> Result<Poll, DomainError> {
        self.created.lock().unwrap().push(cmd.clone());
        Ok(poll_from(cmd))
    }

    async fn set_closed(&self, _id: Uuid, closed: bool) -> Result<bool, DomainError> {
        let mut g = self.stored.lock().unwrap();
        match g.as_mut() {
            Some(p) => {
                p.is_closed = closed;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn delete(&self, _id: Uuid) -> Result<bool, DomainError> {
        Ok(self.stored.lock().unwrap().take().is_some())
    }

    async fn cast_vote(
        &self,
        _poll_id: Uuid,
        option_id: Uuid,
        user_id: &str,
    ) -> Result<bool, DomainError> {
        if *self.option_valide.lock().unwrap() != Some(option_id) {
            return Ok(false);
        }
        self.votes.lock().unwrap().push((option_id, user_id.into()));
        Ok(true)
    }

    async fn vote_of(&self, _poll_id: Uuid, user_id: &str) -> Result<Option<Uuid>, DomainError> {
        Ok(self
            .votes
            .lock()
            .unwrap()
            .iter()
            .find(|(_, u)| u == user_id)
            .map(|(o, _)| *o))
    }
}

fn poll_from(cmd: &UpsertPollCommand) -> Poll {
    Poll {
        id: Uuid::nil(),
        guild_id: cmd.guild_id.clone(),
        question: cmd.question.clone(),
        description: cmd.description.clone(),
        closes_at: cmd.closes_at,
        is_closed: false,
        is_public: cmd.is_public,
        created_by: cmd.created_by.clone(),
        created_at: Utc::now(),
        options: cmd
            .options
            .iter()
            .enumerate()
            .map(|(i, (label, color))| PollOption {
                id: Uuid::nil(),
                label: label.clone(),
                color: color.clone(),
                position: i as i32,
                votes: 0,
            })
            .collect(),
    }
}

fn cmd(options: Vec<(&str, Option<&str>)>) -> UpsertPollCommand {
    UpsertPollCommand {
        guild_id: "g".into(),
        question: "  Quel jeu ?  ".into(),
        description: Some("   ".into()),
        closes_at: Utc::now() + Duration::days(2),
        is_public: true,
        created_by: "staff".into(),
        options: options
            .into_iter()
            .map(|(l, c)| (l.to_string(), c.map(str::to_string)))
            .collect(),
    }
}

fn ouvert(option_id: Uuid) -> Poll {
    Poll {
        id: Uuid::nil(),
        guild_id: "g".into(),
        question: "Quel jeu ?".into(),
        description: None,
        closes_at: Utc::now() + Duration::days(1),
        is_closed: false,
        is_public: true,
        created_by: "staff".into(),
        created_at: Utc::now(),
        options: vec![PollOption {
            id: option_id,
            label: "Enshrouded".into(),
            color: None,
            position: 0,
            votes: 0,
        }],
    }
}

fn service(repo: MockRepo) -> (ManagePollsService, Arc<MockRepo>) {
    let repo = Arc::new(repo);
    (ManagePollsService::new(repo.clone()), repo)
}

// ── Normalisation ──

#[tokio::test]
async fn creation_rogne_la_question_et_vide_la_description_blanche() {
    let (svc, repo) = service(MockRepo::default());
    svc.create(cmd(vec![("A", None), ("B", None)]))
        .await
        .unwrap();

    let saved = repo.created.lock().unwrap()[0].clone();
    assert_eq!(saved.question, "Quel jeu ?");
    assert!(saved.description.is_none());
}

#[tokio::test]
async fn question_vide_est_refusee() {
    let (svc, _) = service(MockRepo::default());
    let mut c = cmd(vec![("A", None), ("B", None)]);
    c.question = "  ".into();
    assert!(matches!(
        svc.create(c).await,
        Err(DomainError::ValidationError(_))
    ));
}

/// Un sondage a un seul choix n'est pas un sondage.
#[tokio::test]
async fn moins_de_deux_choix_est_refuse() {
    let (svc, _) = service(MockRepo::default());
    assert!(matches!(
        svc.create(cmd(vec![("A", None)])).await,
        Err(DomainError::ValidationError(_))
    ));
}

/// Les options vides sont nettoyees AVANT d'etre comptees : deux vraies
/// options plus trois lignes blanches font bien deux choix.
#[tokio::test]
async fn options_blanches_sont_ignorees_puis_comptees() {
    let (svc, repo) = service(MockRepo::default());
    svc.create(cmd(vec![
        ("A", None),
        ("  ", None),
        ("B", None),
        ("", None),
    ]))
    .await
    .unwrap();
    assert_eq!(repo.created.lock().unwrap()[0].options.len(), 2);
}

/// ... et une saisie qui ne laisse qu'un seul choix reel est bien refusee.
#[tokio::test]
async fn options_blanches_ne_masquent_pas_un_choix_unique() {
    let (svc, _) = service(MockRepo::default());
    assert!(matches!(
        svc.create(cmd(vec![("A", None), ("   ", None)])).await,
        Err(DomainError::ValidationError(_))
    ));
}

/// Deux options identiques rendraient le resultat inexploitable.
#[tokio::test]
async fn options_en_double_sont_refusees_meme_a_la_casse_pres() {
    let (svc, _) = service(MockRepo::default());
    assert!(matches!(
        svc.create(cmd(vec![("Valheim", None), ("valheim", None)]))
            .await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn trop_d_options_est_refuse() {
    let (svc, _) = service(MockRepo::default());
    // Libelles distincts, sinon c'est le doublon qui serait detecte d'abord.
    let mut c = cmd(vec![]);
    c.options = (0..12).map(|i| (format!("O{i}"), None)).collect();
    assert!(matches!(
        svc.create(c).await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn couleur_est_normalisee_et_le_diese_retire() {
    let (svc, repo) = service(MockRepo::default());
    svc.create(cmd(vec![
        ("A", Some("#A855F7")),
        ("B", Some("pas-une-couleur")),
    ]))
    .await
    .unwrap();

    let options = repo.created.lock().unwrap()[0].options.clone();
    assert_eq!(options[0].1.as_deref(), Some("a855f7"));
    // Invalide => absente, la palette de repli prendra le relais.
    assert!(options[1].1.is_none());
}

/// Un sondage deja clos a sa creation ne recueillerait aucune voix.
#[tokio::test]
async fn cloture_dans_le_passe_est_refusee() {
    let (svc, _) = service(MockRepo::default());
    let mut c = cmd(vec![("A", None), ("B", None)]);
    c.closes_at = Utc::now() - Duration::hours(1);
    assert!(matches!(
        svc.create(c).await,
        Err(DomainError::ValidationError(_))
    ));
}

// ── Vote ──

#[tokio::test]
async fn vote_valide_est_enregistre() {
    let opt = Uuid::from_u128(7);
    let (svc, repo) = service(MockRepo::with(ouvert(opt), Some(opt)));

    svc.vote(Uuid::nil(), opt, "nowen").await.unwrap();
    assert_eq!(repo.votes.lock().unwrap().len(), 1);
}

/// Sans ce controle, un client pourrait voter pour l'option d'un autre
/// sondage et fausser deux resultats a la fois.
#[tokio::test]
async fn vote_pour_une_option_etrangere_est_refuse() {
    let opt = Uuid::from_u128(7);
    let (svc, repo) = service(MockRepo::with(ouvert(opt), Some(opt)));

    let etrangere = Uuid::from_u128(99);
    assert!(matches!(
        svc.vote(Uuid::nil(), etrangere, "nowen").await,
        Err(DomainError::ValidationError(_))
    ));
    assert!(repo.votes.lock().unwrap().is_empty());
}

#[tokio::test]
async fn vote_sur_un_sondage_clos_manuellement_est_refuse() {
    let opt = Uuid::from_u128(7);
    let mut p = ouvert(opt);
    p.is_closed = true;
    let (svc, _) = service(MockRepo::with(p, Some(opt)));

    assert!(matches!(
        svc.vote(Uuid::nil(), opt, "nowen").await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn vote_apres_la_date_de_cloture_est_refuse() {
    let opt = Uuid::from_u128(7);
    let mut p = ouvert(opt);
    p.closes_at = Utc::now() - Duration::hours(1);
    let (svc, _) = service(MockRepo::with(p, Some(opt)));

    assert!(matches!(
        svc.vote(Uuid::nil(), opt, "nowen").await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn consultation_remonte_le_vote_du_lecteur() {
    let opt = Uuid::from_u128(7);
    let (svc, _) = service(MockRepo::with(ouvert(opt), Some(opt)));
    svc.vote(Uuid::nil(), opt, "nowen").await.unwrap();

    let vu = svc.get(Uuid::nil(), Some("nowen")).await.unwrap();
    assert_eq!(vu.my_vote, Some(opt));
}

/// Un visiteur non connecte n'a pas de vote : la page doit pouvoir afficher
/// les resultats sans identite.
#[tokio::test]
async fn consultation_anonyme_ne_remonte_aucun_vote() {
    let opt = Uuid::from_u128(7);
    let (svc, _) = service(MockRepo::with(ouvert(opt), Some(opt)));

    let vu = svc.get(Uuid::nil(), None).await.unwrap();
    assert!(vu.my_vote.is_none());
}

#[tokio::test]
async fn sondage_introuvable_remonte_une_erreur_claire() {
    let (svc, _) = service(MockRepo::default());
    assert!(matches!(
        svc.get(Uuid::nil(), None).await,
        Err(DomainError::NotFound(_))
    ));
}
