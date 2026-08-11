//! Stockage des sessions web et du journal des logins.
//!
//! Tout le SQL vit dans l'adapter (`auth-api`). La table `web_oauth_sessions`
//! et le journal `successful_logins` appartiennent désormais à la base de
//! l'identité, plus à `discord_sentinel`.
//!
//! La clé d'une session est son **identifiant**, celui que porte le cookie
//! `ds_session` — pas l'access token. L'access token tourne à chaque refresh ;
//! la session, elle, survit. Indexer sur le token ferait perdre la session à
//! chaque rotation, c'est-à-dire perdre le « rester connecté ».

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::session::{
    LoginTrace, NewOAuthSession, OAuthSession, SessionTokenUpdate, SuccessfulLogin,
};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create_session(&self, session: &NewOAuthSession) -> Result<(), DomainError>;

    /// `None` si inconnue — cas normal d'une session expirée ou révoquée, pas
    /// une erreur.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<OAuthSession>, DomainError>;

    async fn update_tokens(&self, update: &SessionTokenUpdate) -> Result<(), DomainError>;

    /// Marque la session comme utilisée (`last_used_at`). Best-effort : sert au
    /// ménage des sessions dormantes, jamais à l'autorisation.
    async fn touch(&self, id: Uuid) -> Result<(), DomainError>;

    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Trace best-effort : un échec d'écriture ne doit jamais faire échouer un
    /// login par ailleurs valide. C'est l'appelant qui absorbe l'erreur.
    async fn record_login(&self, trace: &LoginTrace) -> Result<(), DomainError>;

    async fn list_recent_logins(&self, limit: i64) -> Result<Vec<SuccessfulLogin>, DomainError>;

    /// Purge du journal, appelée par l'écran de sécurité de l'exploitation.
    /// Retourne le nombre de lignes supprimées.
    async fn purge_logins_older_than(&self, days: i32) -> Result<u64, DomainError>;
}
