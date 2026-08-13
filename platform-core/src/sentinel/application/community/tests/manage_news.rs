use super::*;
use chrono::Utc;
use std::sync::Mutex;

#[derive(Default)]
struct MockRepo {
    created: Mutex<Vec<UpsertNewsCommand>>,
    stored: Mutex<Option<NewsPost>>,
}

#[async_trait]
impl NewsRepository for MockRepo {
    async fn list(
        &self,
        _guild_id: &str,
        _published_only: bool,
        _limit: i64,
    ) -> Result<Vec<NewsPost>, DomainError> {
        Ok(vec![])
    }

    async fn find_by_id(&self, _id: Uuid) -> Result<Option<NewsPost>, DomainError> {
        Ok(self.stored.lock().unwrap().clone())
    }

    async fn create(&self, cmd: &UpsertNewsCommand) -> Result<NewsPost, DomainError> {
        self.created.lock().unwrap().push(cmd.clone());
        Ok(news_from(cmd))
    }

    async fn update(
        &self,
        _id: Uuid,
        cmd: &UpsertNewsCommand,
    ) -> Result<Option<NewsPost>, DomainError> {
        self.created.lock().unwrap().push(cmd.clone());
        Ok(self.stored.lock().unwrap().as_ref().map(|_| news_from(cmd)))
    }

    async fn delete(&self, _id: Uuid) -> Result<bool, DomainError> {
        Ok(self.stored.lock().unwrap().take().is_some())
    }
}

fn news_from(cmd: &UpsertNewsCommand) -> NewsPost {
    let now = Utc::now();
    NewsPost {
        id: Uuid::nil(),
        guild_id: cmd.guild_id.clone(),
        title: cmd.title.clone(),
        body: cmd.body.clone(),
        image_url: cmd.image_url.clone(),
        is_pinned: cmd.is_pinned,
        is_public: cmd.is_public,
        published_at: cmd.published_at.unwrap_or(now),
        created_by: cmd.created_by.clone(),
        created_at: now,
    }
}

fn cmd() -> UpsertNewsCommand {
    UpsertNewsCommand {
        guild_id: "g".into(),
        title: "  Le serveur passe en 1.21  ".into(),
        body: "  Sauvegarde faite.  ".into(),
        image_url: Some("  /imgs/annonce_staff.jpg  ".into()),
        is_pinned: false,
        is_public: true,
        published_at: None,
        created_by: "staff".into(),
    }
}

fn service(repo: MockRepo) -> (ManageNewsService, Arc<MockRepo>) {
    let repo = Arc::new(repo);
    (ManageNewsService::new(repo.clone()), repo)
}

#[tokio::test]
async fn creation_rogne_les_champs() {
    let (svc, repo) = service(MockRepo::default());
    svc.create(cmd()).await.unwrap();

    let saved = repo.created.lock().unwrap()[0].clone();
    assert_eq!(saved.title, "Le serveur passe en 1.21");
    assert_eq!(saved.body, "Sauvegarde faite.");
    assert_eq!(saved.image_url.as_deref(), Some("/imgs/annonce_staff.jpg"));
}

#[tokio::test]
async fn titre_vide_est_refuse() {
    let (svc, _) = service(MockRepo::default());
    let mut c = cmd();
    c.title = "  ".into();
    assert!(matches!(
        svc.create(c).await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn texte_vide_est_refuse() {
    let (svc, _) = service(MockRepo::default());
    let mut c = cmd();
    c.body = "   ".into();
    assert!(matches!(
        svc.create(c).await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn image_absente_reste_absente() {
    let (svc, repo) = service(MockRepo::default());
    let mut c = cmd();
    c.image_url = Some("   ".into());
    svc.create(c).await.unwrap();
    assert!(repo.created.lock().unwrap()[0].image_url.is_none());
}

/// Une URL absolue figerait le domaine en base — meme choix que pour les
/// jaquettes de jeu.
#[tokio::test]
async fn url_absolue_est_refusee() {
    let (svc, _) = service(MockRepo::default());
    let mut c = cmd();
    c.image_url = Some("https://ailleurs.example/x.jpg".into());
    assert!(matches!(
        svc.create(c).await,
        Err(DomainError::ValidationError(_))
    ));
}

/// Le cas qui compte vraiment : sans ce filtre, la chaine finirait dans un
/// attribut `src` du site.
#[tokio::test]
async fn schema_javascript_est_refuse() {
    let (svc, _) = service(MockRepo::default());
    let mut c = cmd();
    c.image_url = Some("javascript:alert(1)".into());
    assert!(matches!(
        svc.create(c).await,
        Err(DomainError::ValidationError(_))
    ));
}

/// Un chemin protocol-relative (`//hote/x.jpg`) commence bien par `/` mais
/// pointe vers un autre domaine : il ne doit pas passer.
#[tokio::test]
async fn chemin_protocol_relative_est_refuse() {
    let (svc, _) = service(MockRepo::default());
    let mut c = cmd();
    c.image_url = Some("//evil.example/x.jpg".into());
    assert!(matches!(
        svc.create(c).await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn mise_a_jour_d_une_nouvelle_absente_remonte_not_found() {
    let (svc, _) = service(MockRepo::default());
    assert!(matches!(
        svc.update(Uuid::nil(), cmd()).await,
        Err(DomainError::NotFound(_))
    ));
}

#[tokio::test]
async fn suppression_d_une_nouvelle_absente_remonte_not_found() {
    let (svc, _) = service(MockRepo::default());
    assert!(matches!(
        svc.delete(Uuid::nil()).await,
        Err(DomainError::NotFound(_))
    ));
}
