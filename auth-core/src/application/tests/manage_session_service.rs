//! Les invariants qui, s'ils cassent, déconnectent tout le monde ou laissent
//! entrer n'importe qui.

use super::*;
use crate::domain::entities::identity::DiscordUser;
use crate::domain::entities::session::{LoginTrace, OAuthSession};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
struct FakeSessions {
    stored: Mutex<HashMap<Uuid, OAuthSession>>,
    traces: Mutex<u32>,
    touched: Mutex<u32>,
}

#[async_trait]
impl SessionRepository for FakeSessions {
    async fn create_session(&self, s: &NewOAuthSession) -> Result<(), DomainError> {
        self.stored.lock().unwrap().insert(
            s.id,
            OAuthSession {
                id: s.id,
                discord_user_id: s.discord_user_id.clone(),
                username: s.username.clone(),
                global_name: s.global_name.clone(),
                avatar: s.avatar.clone(),
                access_token: s.access_token.clone(),
                refresh_token: s.refresh_token.clone(),
                access_expires_at: s.access_expires_at,
            },
        );
        Ok(())
    }
    async fn find_by_id(&self, id: Uuid) -> Result<Option<OAuthSession>, DomainError> {
        Ok(self.stored.lock().unwrap().get(&id).cloned())
    }
    async fn update_tokens(&self, u: &SessionTokenUpdate) -> Result<(), DomainError> {
        if let Some(s) = self.stored.lock().unwrap().get_mut(&u.id) {
            s.access_token = u.access_token.clone();
            s.refresh_token = u.refresh_token.clone();
            s.access_expires_at = u.access_expires_at;
        }
        Ok(())
    }
    async fn touch(&self, _id: Uuid) -> Result<(), DomainError> {
        *self.touched.lock().unwrap() += 1;
        Ok(())
    }
    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        self.stored.lock().unwrap().remove(&id);
        Ok(())
    }
    async fn record_login(&self, _t: &LoginTrace) -> Result<(), DomainError> {
        *self.traces.lock().unwrap() += 1;
        Ok(())
    }
    async fn list_recent_logins(&self, _l: i64) -> Result<Vec<SuccessfulLogin>, DomainError> {
        Ok(vec![])
    }
    async fn purge_logins_older_than(&self, _d: i32) -> Result<u64, DomainError> {
        Ok(0)
    }
}

/// Journal indisponible : `record_login` echoue systematiquement.
struct BrokenTraceSessions(FakeSessions);

#[async_trait]
impl SessionRepository for BrokenTraceSessions {
    async fn create_session(&self, s: &NewOAuthSession) -> Result<(), DomainError> {
        self.0.create_session(s).await
    }
    async fn find_by_id(&self, id: Uuid) -> Result<Option<OAuthSession>, DomainError> {
        self.0.find_by_id(id).await
    }
    async fn update_tokens(&self, u: &SessionTokenUpdate) -> Result<(), DomainError> {
        self.0.update_tokens(u).await
    }
    async fn touch(&self, id: Uuid) -> Result<(), DomainError> {
        self.0.touch(id).await
    }
    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        self.0.delete(id).await
    }
    async fn record_login(&self, _t: &LoginTrace) -> Result<(), DomainError> {
        Err(DomainError::Internal("journal hs".into()))
    }
    async fn list_recent_logins(&self, _l: i64) -> Result<Vec<SuccessfulLogin>, DomainError> {
        Ok(vec![])
    }
    async fn purge_logins_older_than(&self, _d: i32) -> Result<u64, DomainError> {
        Ok(0)
    }
}

struct FakeDiscord {
    expires_in: i64,
    refresh_token: String,
    refresh_result: Option<DomainError>,
    refresh_calls: Mutex<u32>,
}

impl Default for FakeDiscord {
    fn default() -> Self {
        Self {
            expires_in: 3600,
            refresh_token: "refresh-1".into(),
            refresh_result: None,
            refresh_calls: Mutex::new(0),
        }
    }
}

#[async_trait]
impl DiscordIdentity for FakeDiscord {
    fn authorize_url(&self, state: &str) -> String {
        format!("https://discord/authorize?state={state}")
    }
    async fn exchange_code(&self, _code: &str) -> Result<TokenPair, DomainError> {
        Ok(TokenPair {
            access_token: "access-1".into(),
            refresh_token: self.refresh_token.clone(),
            expires_in_secs: self.expires_in,
        })
    }
    async fn refresh(&self, _refresh_token: &str) -> Result<TokenPair, DomainError> {
        *self.refresh_calls.lock().unwrap() += 1;
        match &self.refresh_result {
            Some(DomainError::Forbidden(m)) => Err(DomainError::Forbidden(m.clone())),
            Some(_) => Err(DomainError::Internal("discord hs".into())),
            None => Ok(TokenPair {
                access_token: "access-2".into(),
                refresh_token: "refresh-2".into(),
                expires_in_secs: 3600,
            }),
        }
    }
    async fn get_user_me(&self, _access_token: &str) -> Result<DiscordUser, DomainError> {
        Ok(DiscordUser {
            id: "42".into(),
            username: "moi".into(),
            global_name: None,
            avatar: None,
        })
    }
}

#[derive(Default)]
struct FakeStates {
    valid: Mutex<Vec<String>>,
}

#[async_trait]
impl LoginStateStore for FakeStates {
    async fn put(&self, state: &str, _ttl: u64) -> Result<(), DomainError> {
        self.valid.lock().unwrap().push(state.to_string());
        Ok(())
    }
    async fn take(&self, state: &str) -> Result<bool, DomainError> {
        let mut v = self.valid.lock().unwrap();
        match v.iter().position(|s| s == state) {
            Some(i) => {
                v.remove(i);
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

fn service(
    sessions: Arc<dyn SessionRepository>,
    discord: Arc<FakeDiscord>,
    states: Arc<FakeStates>,
    allowed: &[&str],
) -> ManageSessionService {
    ManageSessionService {
        sessions,
        discord,
        states,
        policy: SuperadminPolicy::new(allowed.iter().map(|s| s.to_string()).collect()),
        new_state: || "state-fixe".to_string(),
    }
}

#[tokio::test]
async fn un_state_ne_sert_qu_une_fois() {
    let states = Arc::new(FakeStates::default());
    let svc = service(
        Arc::new(FakeSessions::default()),
        Arc::new(FakeDiscord::default()),
        states,
        &["42"],
    );
    svc.start_login().await.unwrap();
    assert!(svc
        .complete_login("code", "state-fixe", LoginContext::default())
        .await
        .is_ok());
    // Rejeu du meme state : refuse.
    assert!(svc
        .complete_login("code", "state-fixe", LoginContext::default())
        .await
        .is_err());
}

#[tokio::test]
async fn un_state_inconnu_est_refuse_sans_appeler_discord() {
    let svc = service(
        Arc::new(FakeSessions::default()),
        Arc::new(FakeDiscord::default()),
        Arc::new(FakeStates::default()),
        &["42"],
    );
    assert!(svc
        .complete_login("code", "jamais-emis", LoginContext::default())
        .await
        .is_err());
}

/// Le journal de securite est un confort ; il ne doit jamais bloquer un login.
#[tokio::test]
async fn un_journal_en_panne_ne_bloque_pas_le_login() {
    let states = Arc::new(FakeStates::default());
    let svc = service(
        Arc::new(BrokenTraceSessions(FakeSessions::default())),
        Arc::new(FakeDiscord::default()),
        states,
        &["42"],
    );
    svc.start_login().await.unwrap();
    let session = svc
        .complete_login("code", "state-fixe", LoginContext::default())
        .await
        .unwrap();
    assert!(session.session_id.is_some());
}

/// Sans refresh token, le login reussit mais sans « rester connecte ».
#[tokio::test]
async fn sans_refresh_token_pas_de_session_persistante() {
    let states = Arc::new(FakeStates::default());
    let discord = Arc::new(FakeDiscord {
        refresh_token: String::new(),
        ..Default::default()
    });
    let svc = service(Arc::new(FakeSessions::default()), discord, states, &["42"]);
    svc.start_login().await.unwrap();
    let session = svc
        .complete_login("code", "state-fixe", LoginContext::default())
        .await
        .unwrap();
    assert!(session.session_id.is_none());
    assert_eq!(session.access_token, "access-1");
}

#[tokio::test]
async fn is_superadmin_suit_la_liste() {
    let states = Arc::new(FakeStates::default());
    let svc = service(
        Arc::new(FakeSessions::default()),
        Arc::new(FakeDiscord::default()),
        states,
        &["999"],
    );
    svc.start_login().await.unwrap();
    let session = svc
        .complete_login("code", "state-fixe", LoginContext::default())
        .await
        .unwrap();
    assert!(!session.is_superadmin);
}

/// Un token encore valide ne doit pas declencher d'appel a Discord : sinon
/// chaque rechargement de page consomme du rate limit.
#[tokio::test]
async fn un_token_valide_est_rendu_sans_appeler_discord() {
    let sessions = Arc::new(FakeSessions::default());
    let discord = Arc::new(FakeDiscord::default());
    let states = Arc::new(FakeStates::default());
    let svc = service(sessions.clone(), discord.clone(), states, &["42"]);
    svc.start_login().await.unwrap();
    let id = svc
        .complete_login("code", "state-fixe", LoginContext::default())
        .await
        .unwrap()
        .session_id
        .unwrap();

    let refreshed = svc.refresh(id).await.unwrap();
    assert_eq!(refreshed.access_token, "access-1");
    assert_eq!(*discord.refresh_calls.lock().unwrap(), 0);
    assert_eq!(*sessions.touched.lock().unwrap(), 1);
}

/// Token expire : on rafraichit, et la session porte le nouveau couple.
#[tokio::test]
async fn un_token_expire_est_rafraichi() {
    let sessions = Arc::new(FakeSessions::default());
    let discord = Arc::new(FakeDiscord {
        // Sous la marge de rafraichissement (60 s) : la session est a bout de
        // course des sa creation. Une valeur negative tomberait dans le repli
        // « Discord n'a pas annonce de duree » et donnerait 7 jours.
        expires_in: 1,
        ..Default::default()
    });
    let states = Arc::new(FakeStates::default());
    let svc = service(sessions.clone(), discord.clone(), states, &["42"]);
    svc.start_login().await.unwrap();
    let id = svc
        .complete_login("code", "state-fixe", LoginContext::default())
        .await
        .unwrap()
        .session_id
        .unwrap();

    let refreshed = svc.refresh(id).await.unwrap();
    assert_eq!(refreshed.access_token, "access-2");
    assert_eq!(*discord.refresh_calls.lock().unwrap(), 1);
    let stored = sessions.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(stored.refresh_token, "refresh-2");
}

/// LA distinction a ne pas perdre : un refus Discord invalide la session
/// (application revoquee), une panne reseau ne doit deconnecter personne.
#[tokio::test]
async fn un_refus_discord_invalide_la_session_mais_pas_une_panne() {
    for (erreur, doit_survivre) in [
        (DomainError::Forbidden("revoque".into()), false),
        (DomainError::Internal("reseau".into()), true),
    ] {
        let sessions = Arc::new(FakeSessions::default());
        let discord = Arc::new(FakeDiscord {
            // Sous la marge de rafraichissement (60 s) : la session est a bout de
            // course des sa creation. Une valeur negative tomberait dans le repli
            // « Discord n'a pas annonce de duree » et donnerait 7 jours.
            expires_in: 1,
            refresh_result: Some(erreur),
            ..Default::default()
        });
        let states = Arc::new(FakeStates::default());
        let svc = service(sessions.clone(), discord, states, &["42"]);
        svc.start_login().await.unwrap();
        let id = svc
            .complete_login("code", "state-fixe", LoginContext::default())
            .await
            .unwrap()
            .session_id
            .unwrap();

        assert!(svc.refresh(id).await.is_err());
        assert_eq!(
            sessions.find_by_id(id).await.unwrap().is_some(),
            doit_survivre
        );
    }
}

#[tokio::test]
async fn le_logout_est_idempotent() {
    let svc = service(
        Arc::new(FakeSessions::default()),
        Arc::new(FakeDiscord::default()),
        Arc::new(FakeStates::default()),
        &["42"],
    );
    assert!(svc.logout(Uuid::new_v4()).await.is_ok());
}
