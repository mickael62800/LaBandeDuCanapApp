//! Redis : `state` CSRF du flux OAuth et cache « jeton → identité ».
//!
//! Deux ports, un seul client : ce sont deux usages du même Redis, mais des
//! contrats différents — le `state` DOIT être fiable (sa perte casse un login),
//! le cache est best-effort (sa perte coûte de la latence).

use async_trait::async_trait;
use redis::AsyncCommands;

use auth_core::domain::errors::DomainError;
use auth_core::ports::outbound::identity_cache::IdentityCache;
use auth_core::ports::outbound::login_state_store::LoginStateStore;

const STATE_PREFIX: &str = "oauth:web:state:";

pub struct RedisLoginStateStore {
    client: redis::Client,
}

impl RedisLoginStateStore {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl LoginStateStore for RedisLoginStateStore {
    async fn put(&self, state: &str, ttl_secs: u64) -> Result<(), DomainError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| {
                tracing::error!(%error, "Redis indisponible pour le state OAuth");
                DomainError::Internal("Redis indisponible".into())
            })?;

        conn.set_ex::<_, _, ()>(format!("{STATE_PREFIX}{state}"), "1", ttl_secs)
            .await
            .map_err(|error| {
                tracing::error!(%error, "ecriture du state OAuth impossible");
                DomainError::Internal("Redis indisponible".into())
            })
    }

    async fn take(&self, state: &str) -> Result<bool, DomainError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| {
                tracing::error!(%error, "Redis indisponible pour le state OAuth");
                DomainError::Internal("Redis indisponible".into())
            })?;

        // GETDEL et non GET puis DEL : un GET suivi d'un DEL laisse une fenetre
        // ou deux callbacks concurrents portant le meme state passent tous les
        // deux. C'est ce qui fait du state une protection contre le REJEU et
        // pas seulement contre le CSRF.
        let existed: Option<String> = conn
            .get_del(format!("{STATE_PREFIX}{state}"))
            .await
            .map_err(|error| {
                tracing::error!(%error, "consommation du state OAuth impossible");
                DomainError::Internal("Redis indisponible".into())
            })?;

        Ok(existed.is_some())
    }
}

pub struct RedisIdentityCache {
    client: redis::Client,
}

impl RedisIdentityCache {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl IdentityCache for RedisIdentityCache {
    async fn get(&self, key: &str) -> Result<Option<String>, DomainError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| DomainError::Internal("cache indisponible".into()))?;
        conn.get::<_, Option<String>>(key)
            .await
            .map_err(|_| DomainError::Internal("cache indisponible".into()))
    }

    async fn put(&self, key: &str, discord_user_id: &str, ttl_secs: u64) {
        // Best-effort assume : une panne du cache doit coûter de la latence,
        // jamais l'accès. On avale l'erreur ici plutôt que de la remonter à un
        // appelant qui n'en ferait rien.
        if let Ok(mut conn) = self.client.get_multiplexed_async_connection().await {
            let _: Result<(), _> = conn.set_ex(key, discord_user_id, ttl_secs).await;
        }
    }
}

/// Dérive une clé de cache opaque depuis un access token — jamais stocké en
/// clair. SHA-256 tronqué à 128 bits : contrairement à un hash non
/// cryptographique, une collision choisie n'est pas calculable, ce qui écarte
/// l'usurpation d'identité par collision de clé (résoudre le token A vers
/// l'identité de B).
pub fn cache_key(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(input.as_bytes());
    digest[..16].iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::cache_key;

    #[test]
    fn la_cle_est_stable() {
        assert_eq!(cache_key("abc"), cache_key("abc"));
    }

    #[test]
    fn deux_jetons_donnent_deux_cles() {
        assert_ne!(cache_key("token-a"), cache_key("token-b"));
    }

    #[test]
    fn la_cle_fait_128_bits_en_hexa() {
        let k = cache_key("whatever");
        assert_eq!(k.len(), 32);
        assert!(k.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// L'invariant qui justifie le hachage : le jeton ne doit jamais se
    /// retrouver en clair dans une cle Redis.
    #[test]
    fn la_cle_ne_laisse_pas_fuir_le_jeton() {
        let token = "super-secret-access-token";
        assert!(!cache_key(token).contains(token));
    }
}
