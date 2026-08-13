use super::*;
use chrono::Duration;
use std::sync::Mutex;

use crate::sentinel::domain::entities::community::lfg::LfgInterest;

/// Repository en memoire : une seule annonce, remplacable par les tests.
#[derive(Default)]
struct MockRepo {
    stored: Mutex<Option<LfgPost>>,
    created: Mutex<Vec<UpsertLfgCommand>>,
    interests: Mutex<Vec<(String, String)>>,
    removed: Mutex<Vec<String>>,
}

impl MockRepo {
    fn with(post: LfgPost) -> Self {
        Self {
            stored: Mutex::new(Some(post)),
            ..Default::default()
        }
    }
}

#[async_trait]
impl LfgRepository for MockRepo {
    async fn list(
        &self,
        _guild_id: &str,
        _live_only: bool,
        _limit: i64,
    ) -> Result<Vec<LfgPost>, DomainError> {
        Ok(self.stored.lock().unwrap().clone().into_iter().collect())
    }

    async fn find_by_id(&self, _id: Uuid) -> Result<Option<LfgPost>, DomainError> {
        Ok(self.stored.lock().unwrap().clone())
    }

    async fn create(&self, cmd: &UpsertLfgCommand) -> Result<LfgPost, DomainError> {
        self.created.lock().unwrap().push(cmd.clone());
        Ok(post_from(cmd))
    }

    async fn set_open(&self, _id: Uuid, open: bool) -> Result<bool, DomainError> {
        if let Some(p) = self.stored.lock().unwrap().as_mut() {
            p.is_open = open;
        }
        Ok(true)
    }

    async fn delete(&self, _id: Uuid) -> Result<bool, DomainError> {
        *self.stored.lock().unwrap() = None;
        Ok(true)
    }

    async fn add_interest(
        &self,
        _id: Uuid,
        user_id: &str,
        username: &str,
    ) -> Result<(), DomainError> {
        self.interests
            .lock()
            .unwrap()
            .push((user_id.to_string(), username.to_string()));
        if let Some(p) = self.stored.lock().unwrap().as_mut() {
            p.interested.push(LfgInterest {
                user_id: user_id.to_string(),
                username: username.to_string(),
                joined_at: Utc::now(),
            });
        }
        Ok(())
    }

    async fn remove_interest(&self, _id: Uuid, user_id: &str) -> Result<bool, DomainError> {
        self.removed.lock().unwrap().push(user_id.to_string());
        if let Some(p) = self.stored.lock().unwrap().as_mut() {
            p.interested.retain(|i| i.user_id != user_id);
        }
        Ok(true)
    }

    async fn purge_expired(&self, _older_than_hours: i64) -> Result<u64, DomainError> {
        Ok(0)
    }
}

fn post_from(cmd: &UpsertLfgCommand) -> LfgPost {
    let now = Utc::now();
    LfgPost {
        id: Uuid::nil(),
        guild_id: cmd.guild_id.clone(),
        author_id: cmd.author_id.clone(),
        author_name: cmd.author_name.clone(),
        game: cmd.game.clone(),
        game_server_id: cmd.game_server_id,
        slots: cmd.slots,
        when_text: cmd.when_text.clone(),
        description: cmd.description.clone(),
        is_open: true,
        expires_at: cmd.resolved_expiry(now),
        created_at: now,
        interested: vec![],
    }
}

fn cmd() -> UpsertLfgCommand {
    UpsertLfgCommand {
        guild_id: "g".into(),
        author_id: "kalyx".into(),
        author_name: "  Kalyx  ".into(),
        game: "  Valheim  ".into(),
        game_server_id: None,
        slots: 2,
        when_text: "  ce soir 21h  ".into(),
        description: Some("   ".into()),
        expires_at: None,
    }
}

fn vivante(expire_dans_h: i64) -> LfgPost {
    LfgPost {
        id: Uuid::nil(),
        guild_id: "g".into(),
        author_id: "kalyx".into(),
        author_name: "Kalyx".into(),
        game: "Valheim".into(),
        game_server_id: None,
        slots: 2,
        when_text: "ce soir".into(),
        description: None,
        is_open: true,
        expires_at: Utc::now() + Duration::hours(expire_dans_h),
        created_at: Utc::now(),
        interested: vec![],
    }
}

fn service(repo: MockRepo) -> (ManageLfgService, Arc<MockRepo>) {
    let repo = Arc::new(repo);
    (ManageLfgService::new(repo.clone()), repo)
}

// ── Normalisation ──

#[tokio::test]
async fn creation_rogne_les_champs_libres() {
    let (svc, repo) = service(MockRepo::default());
    svc.create(cmd()).await.unwrap();

    let saved = repo.created.lock().unwrap()[0].clone();
    assert_eq!(saved.game, "Valheim");
    assert_eq!(saved.author_name, "Kalyx");
    assert_eq!(saved.when_text, "ce soir 21h");
}

/// Une description blanche doit devenir NULL, pas une chaine vide que le
/// front devrait ensuite distinguer.
#[tokio::test]
async fn description_blanche_devient_absente() {
    let (svc, repo) = service(MockRepo::default());
    svc.create(cmd()).await.unwrap();
    assert!(repo.created.lock().unwrap()[0].description.is_none());
}

/// Plutot que de refuser une annonce sans horaire, on lui donne la
/// formulation qui correspond.
#[tokio::test]
async fn creneau_vide_recoit_une_formulation_par_defaut() {
    let (svc, repo) = service(MockRepo::default());
    let mut c = cmd();
    c.when_text = "   ".into();
    svc.create(c).await.unwrap();
    assert_eq!(
        repo.created.lock().unwrap()[0].when_text,
        "quand vous voulez"
    );
}

#[tokio::test]
async fn jeu_vide_est_refuse() {
    let (svc, _) = service(MockRepo::default());
    let mut c = cmd();
    c.game = "   ".into();
    assert!(matches!(
        svc.create(c).await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn nombre_de_places_hors_bornes_est_refuse() {
    let (svc, _) = service(MockRepo::default());
    for n in [0, -3, 51, 9999] {
        let mut c = cmd();
        c.slots = n;
        assert!(
            matches!(svc.create(c).await, Err(DomainError::ValidationError(_))),
            "slots = {n} aurait du etre refuse"
        );
    }
}

// ── Propriete de l'annonce ──

#[tokio::test]
async fn l_auteur_peut_fermer_son_annonce() {
    let (svc, _) = service(MockRepo::with(vivante(5)));
    assert!(svc.close(Uuid::nil(), "kalyx", false).await.is_ok());
}

/// Sans ce controle, n'importe qui fermerait l'annonce d'un autre.
#[tokio::test]
async fn un_tiers_ne_peut_pas_fermer_l_annonce_d_un_autre() {
    let (svc, _) = service(MockRepo::with(vivante(5)));
    assert!(matches!(
        svc.close(Uuid::nil(), "intrus", false).await,
        Err(DomainError::Forbidden(_))
    ));
}

#[tokio::test]
async fn le_staff_peut_moderer_l_annonce_d_un_autre() {
    let (svc, _) = service(MockRepo::with(vivante(5)));
    assert!(svc.close(Uuid::nil(), "moderateur", true).await.is_ok());
}

#[tokio::test]
async fn suppression_suit_la_meme_regle_de_propriete() {
    let (svc, _) = service(MockRepo::with(vivante(5)));
    assert!(matches!(
        svc.delete(Uuid::nil(), "intrus", false).await,
        Err(DomainError::Forbidden(_))
    ));
}

// ── Participation ──

#[tokio::test]
async fn se_manifester_enregistre_le_membre_et_rogne_son_nom() {
    let (svc, repo) = service(MockRepo::with(vivante(5)));
    let post = svc.join(Uuid::nil(), "nowen", "  Nowen  ").await.unwrap();

    assert_eq!(repo.interests.lock().unwrap()[0].1, "Nowen");
    // La relecture doit refleter l'ajout : le front affiche les avatars a
    // partir de cette reponse.
    assert_eq!(post.interested.len(), 1);
    assert_eq!(post.remaining_slots(), 1);
}

/// Sinon « cherche 2 joueurs » afficherait 1 place restante des la creation.
#[tokio::test]
async fn l_auteur_ne_peut_pas_se_compter_lui_meme() {
    let (svc, _) = service(MockRepo::with(vivante(5)));
    assert!(matches!(
        svc.join(Uuid::nil(), "kalyx", "Kalyx").await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn se_manifester_sur_une_annonce_fermee_est_refuse() {
    let mut p = vivante(5);
    p.is_open = false;
    let (svc, _) = service(MockRepo::with(p));
    assert!(matches!(
        svc.join(Uuid::nil(), "nowen", "Nowen").await,
        Err(DomainError::ValidationError(_))
    ));
}

/// L'auteur ne regarde plus une annonce expiree : y repondre ne mene nulle
/// part, meme si personne ne l'a fermee.
#[tokio::test]
async fn se_manifester_sur_une_annonce_expiree_est_refuse() {
    let (svc, _) = service(MockRepo::with(vivante(-1)));
    assert!(matches!(
        svc.join(Uuid::nil(), "nowen", "Nowen").await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn se_retirer_met_la_liste_a_jour() {
    let (svc, _) = service(MockRepo::with(vivante(5)));
    svc.join(Uuid::nil(), "nowen", "Nowen").await.unwrap();
    let post = svc.leave(Uuid::nil(), "nowen").await.unwrap();
    assert!(post.interested.is_empty());
}

// ── Bornes de liste ──

#[tokio::test]
async fn limite_de_liste_est_bornee() {
    let (svc, _) = service(MockRepo::default());
    // Ne doit pas paniquer ni laisser passer une limite absurde.
    assert!(svc.list("g", true, 0).await.is_ok());
    assert!(svc.list("g", true, 100_000).await.is_ok());
}

#[tokio::test]
async fn annonce_introuvable_remonte_une_erreur_claire() {
    let (svc, _) = service(MockRepo::default());
    assert!(matches!(
        svc.get(Uuid::nil()).await,
        Err(DomainError::NotFound(_))
    ));
}
