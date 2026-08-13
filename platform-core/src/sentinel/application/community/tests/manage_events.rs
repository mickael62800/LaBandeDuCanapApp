use super::*;
use chrono::{Duration, TimeZone, Utc};
use std::sync::Mutex;

use crate::sentinel::domain::entities::community::event::{EventParticipant, EventStatus};

#[derive(Default)]
struct MockRepo {
    created: Mutex<Vec<UpsertEventCommand>>,
}

#[async_trait]
impl EventRepository for MockRepo {
    async fn list_in_window(
        &self,
        _guild_id: &str,
        _window: EventWindow,
        _public_only: bool,
    ) -> Result<Vec<CommunityEvent>, DomainError> {
        Ok(vec![])
    }

    async fn find_by_id(&self, _id: Uuid) -> Result<Option<CommunityEvent>, DomainError> {
        Ok(None)
    }

    async fn create(&self, cmd: &UpsertEventCommand) -> Result<CommunityEvent, DomainError> {
        self.created.lock().unwrap().push(cmd.clone());
        Ok(CommunityEvent {
            id: Uuid::nil(),
            guild_id: cmd.guild_id.clone(),
            title: cmd.title.clone(),
            description: cmd.description.clone(),
            game: cmd.game.clone(),
            color: cmd.color.clone(),
            starts_at: cmd.starts_at,
            ends_at: cmd.ends_at,
            all_day: cmd.all_day,
            is_public: cmd.is_public,
            status: cmd.status,
            created_by: cmd.created_by.clone(),
            created_at: cmd.starts_at,
            updated_at: cmd.starts_at,
        })
    }

    async fn update(
        &self,
        _id: Uuid,
        _cmd: &UpsertEventCommand,
    ) -> Result<Option<CommunityEvent>, DomainError> {
        Ok(None)
    }

    async fn delete(&self, _id: Uuid) -> Result<bool, DomainError> {
        Ok(false)
    }

    async fn list_participants(
        &self,
        _event_id: Uuid,
    ) -> Result<Vec<EventParticipant>, DomainError> {
        Ok(vec![])
    }

    async fn set_participation(
        &self,
        _event_id: Uuid,
        _user_id: &str,
        _username: &str,
        _answer: EventAnswer,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn remove_participation(
        &self,
        _event_id: Uuid,
        _user_id: &str,
    ) -> Result<bool, DomainError> {
        Ok(true)
    }
}

fn cmd(title: &str, days: i64) -> UpsertEventCommand {
    let start = Utc.with_ymd_and_hms(2026, 2, 1, 20, 0, 0).unwrap();
    UpsertEventCommand {
        guild_id: "g".into(),
        title: title.into(),
        description: None,
        game: None,
        color: None,
        starts_at: start,
        ends_at: start + Duration::days(days),
        all_day: false,
        is_public: true,
        status: EventStatus::Published,
        created_by: "u".into(),
    }
}

fn service() -> (ManageEventsService, Arc<MockRepo>) {
    let repo = Arc::new(MockRepo::default());
    (ManageEventsService::new(repo.clone()), repo)
}

#[tokio::test]
async fn refuse_un_titre_vide() {
    let (svc, _) = service();
    assert!(svc.create(cmd("   ", 1)).await.is_err());
}

#[tokio::test]
async fn refuse_une_fin_avant_le_debut() {
    let (svc, _) = service();
    assert!(svc.create(cmd("Soiree", -1)).await.is_err());
}

#[tokio::test]
async fn refuse_une_duree_absurde() {
    // Typiquement une annee mal saisie : sans garde-fou, l'evenement
    // apparaitrait dans toutes les vues du calendrier pendant des annees.
    let (svc, _) = service();
    assert!(svc.create(cmd("Campagne", 900)).await.is_err());
}

#[tokio::test]
async fn accepte_une_campagne_de_trois_semaines() {
    let (svc, _) = service();
    assert!(svc.create(cmd("Saison Minecraft", 21)).await.is_ok());
}

#[tokio::test]
async fn normalise_les_champs_libres() {
    let (svc, repo) = service();
    let mut c = cmd("  Saison  ", 7);
    c.description = Some("   ".into());
    c.game = Some("  Minecraft  ".into());
    c.color = Some("#A855F7".into());
    svc.create(c).await.unwrap();

    let created = repo.created.lock().unwrap();
    let got = &created[0];
    assert_eq!(got.title, "Saison");
    // Une description vide devient absente : le front n'a pas a distinguer
    // la chaine vide du NULL.
    assert_eq!(got.description, None);
    assert_eq!(got.game.as_deref(), Some("Minecraft"));
    // Couleur normalisee : sans diese, en minuscules.
    assert_eq!(got.color.as_deref(), Some("a855f7"));
}

#[tokio::test]
async fn rejette_une_couleur_invalide() {
    let (svc, repo) = service();
    let mut c = cmd("Soiree", 1);
    c.color = Some("rouge".into());
    svc.create(c).await.unwrap();
    assert_eq!(repo.created.lock().unwrap()[0].color, None);
}

#[tokio::test]
async fn inscription_sur_evenement_inexistant_refusee() {
    // Sans cette garde, la contrainte de cle etrangere remonterait une erreur
    // d'infrastructure illisible pour l'utilisateur.
    let (svc, _) = service();
    let err = svc
        .join(Uuid::nil(), "u1", "Alice", EventAnswer::Going)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}
