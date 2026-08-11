//! « Qui appelle, et a-t-il le droit d'entrer ? »
//!
//! Porté depuis `sentinel-api/.../middleware/superadmin.rs`, dont la logique
//! était juste mais logée dans un middleware HTTP d'une seule plateforme.

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::entities::identity::{AccessVerdict, SuperadminPolicy};
use crate::domain::errors::DomainError;
use crate::ports::inbound::resolve_access::ResolveAccessUseCase;
use crate::ports::outbound::discord_identity::DiscordIdentity;
use crate::ports::outbound::identity_cache::IdentityCache;

const USER_ID_CACHE_TTL_SECS: u64 = 600;

pub struct ResolveAccessService {
    pub discord: Arc<dyn DiscordIdentity>,
    pub cache: Arc<dyn IdentityCache>,
    pub policy: SuperadminPolicy,
    /// Dérive une clé de cache opaque depuis l'access token. Injectée plutôt
    /// que codée ici : le hachage est de l'infra (`sha2`), et le cœur ne doit
    /// pas en dépendre. L'implémentation de référence est un SHA-256 tronqué —
    /// un hash non cryptographique ouvrirait une usurpation par collision
    /// choisie (résoudre le token A vers l'identité de B).
    pub cache_key: fn(&str) -> String,
}

#[async_trait]
impl ResolveAccessUseCase for ResolveAccessService {
    async fn resolve(&self, access_token: &str) -> Result<AccessVerdict, DomainError> {
        if access_token.is_empty() {
            return Err(DomainError::Forbidden("jeton absent".into()));
        }

        let key = format!("user_id:{}", (self.cache_key)(access_token));

        let discord_user_id = match self.cache.get(&key).await {
            Ok(Some(id)) if !id.is_empty() => id,
            // Cache vide OU cache en panne : on retombe sur Discord. Une panne
            // du cache doit coûter de la latence, pas l'accès.
            _ => {
                let user = self.discord.get_user_me(access_token).await?;
                self.cache.put(&key, &user.id, USER_ID_CACHE_TTL_SECS).await;
                user.id
            }
        };

        let granted = self.policy.grants(&discord_user_id);
        if !granted {
            tracing::warn!(
                user_id = %discord_user_id,
                "acces refuse (absent de la liste superadmin)"
            );
        }

        Ok(AccessVerdict {
            discord_user_id,
            granted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::identity::{DiscordUser, TokenPair};
    use std::sync::Mutex;

    struct FakeDiscord {
        user_id: String,
        calls: Mutex<u32>,
        fail: bool,
    }

    #[async_trait]
    impl DiscordIdentity for FakeDiscord {
        fn authorize_url(&self, _state: &str) -> String {
            String::new()
        }
        async fn exchange_code(&self, _code: &str) -> Result<TokenPair, DomainError> {
            unimplemented!()
        }
        async fn refresh(&self, _refresh_token: &str) -> Result<TokenPair, DomainError> {
            unimplemented!()
        }
        async fn get_user_me(&self, _access_token: &str) -> Result<DiscordUser, DomainError> {
            *self.calls.lock().unwrap() += 1;
            if self.fail {
                return Err(DomainError::Internal("Discord injoignable".into()));
            }
            Ok(DiscordUser {
                id: self.user_id.clone(),
                username: "moi".into(),
                global_name: None,
                avatar: None,
            })
        }
    }

    #[derive(Default)]
    struct FakeCache {
        entries: Mutex<std::collections::HashMap<String, String>>,
        /// Simule un cache indisponible : lecture ET écriture inertes.
        broken: bool,
    }

    #[async_trait]
    impl IdentityCache for FakeCache {
        async fn get(&self, key: &str) -> Result<Option<String>, DomainError> {
            if self.broken {
                return Err(DomainError::Internal("cache hs".into()));
            }
            Ok(self.entries.lock().unwrap().get(key).cloned())
        }
        async fn put(&self, key: &str, discord_user_id: &str, _ttl: u64) {
            if self.broken {
                return;
            }
            self.entries
                .lock()
                .unwrap()
                .insert(key.to_string(), discord_user_id.to_string());
        }
    }

    fn service(
        user_id: &str,
        allowed: &[&str],
        cache: Arc<FakeCache>,
        fail: bool,
    ) -> ResolveAccessService {
        ResolveAccessService {
            discord: Arc::new(FakeDiscord {
                user_id: user_id.to_string(),
                calls: Mutex::new(0),
                fail,
            }),
            cache,
            policy: SuperadminPolicy::new(allowed.iter().map(|s| s.to_string()).collect()),
            cache_key: |t| t.to_string(),
        }
    }

    #[tokio::test]
    async fn accorde_un_superadmin() {
        let svc = service("42", &["42"], Arc::new(FakeCache::default()), false);
        let verdict = svc.resolve("tok").await.unwrap();
        assert_eq!(verdict.discord_user_id, "42");
        assert!(verdict.granted);
    }

    #[tokio::test]
    async fn refuse_un_compte_hors_liste() {
        let svc = service("99", &["42"], Arc::new(FakeCache::default()), false);
        let verdict = svc.resolve("tok").await.unwrap();
        assert!(!verdict.granted);
    }

    /// Le point qui compte : Discord injoignable doit remonter une ERREUR, pas
    /// un refus. L'appelant en fait un 503 ; en faire un 403 signalerait a tort
    /// une revocation de droits.
    #[tokio::test]
    async fn discord_injoignable_nest_pas_un_refus() {
        let svc = service("42", &["42"], Arc::new(FakeCache::default()), true);
        assert!(svc.resolve("tok").await.is_err());
    }

    /// Un cache en panne ne doit pas fermer le back-office : on retombe sur
    /// Discord.
    #[tokio::test]
    async fn cache_en_panne_ne_ferme_pas_lacces() {
        let cache = Arc::new(FakeCache {
            entries: Mutex::new(std::collections::HashMap::new()),
            broken: true,
        });
        let svc = service("42", &["42"], cache, false);
        assert!(svc.resolve("tok").await.unwrap().granted);
    }

    #[tokio::test]
    async fn le_second_appel_passe_par_le_cache() {
        let cache = Arc::new(FakeCache::default());
        let svc = service("42", &["42"], cache.clone(), false);
        svc.resolve("tok").await.unwrap();
        svc.resolve("tok").await.unwrap();
        assert_eq!(cache.entries.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn jeton_vide_refuse_sans_appeler_discord() {
        let svc = service("42", &["42"], Arc::new(FakeCache::default()), true);
        assert!(svc.resolve("").await.is_err());
    }
}
