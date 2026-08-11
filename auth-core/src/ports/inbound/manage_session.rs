//! Port inbound : cycle de vie d'une session web (login, refresh, logout).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::session::{LoginTrace, SuccessfulLogin};
use crate::domain::errors::DomainError;

/// Ce que le front reçoit à l'issue d'un login ou d'un refresh.
///
/// `is_superadmin` n'est PAS une autorisation : c'est un confort d'affichage
/// (le front décide s'il propose le lien back-office). L'autorisation réelle
/// est tranchée à chaque requête par `ResolveAccessUseCase`. Confondre les deux
/// reviendrait à laisser le client décider de ses propres droits.
#[derive(Debug, Clone)]
pub struct EstablishedSession {
    /// `None` quand Discord n'a pas rendu de refresh token : le login réussit,
    /// mais sans persistance — l'utilisateur devra se reconnecter à
    /// l'expiration. C'est le comportement historique, conservé tel quel.
    pub session_id: Option<Uuid>,
    pub access_token: String,
    pub discord_user_id: String,
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
    pub is_superadmin: bool,
}

/// Contexte de la requête de login, pour la trace. Séparé de `LoginTrace` :
/// l'identité n'est connue qu'après l'échange.
#[derive(Debug, Clone, Default)]
pub struct LoginContext {
    pub client_ip: String,
    pub user_agent: String,
}

impl LoginContext {
    pub fn into_trace(self, discord_user_id: String, username: String) -> LoginTrace {
        LoginTrace {
            discord_user_id,
            username,
            client_ip: self.client_ip,
            user_agent: self.user_agent,
        }
    }
}

#[async_trait]
pub trait ManageSessionUseCase: Send + Sync {
    /// Démarre un login : génère le `state` CSRF et rend l'URL Discord.
    async fn start_login(&self) -> Result<String, DomainError>;

    /// Termine le login : vérifie le `state`, échange le code, persiste la
    /// session et journalise la trace.
    async fn complete_login(
        &self,
        code: &str,
        state: &str,
        context: LoginContext,
    ) -> Result<EstablishedSession, DomainError>;

    /// Prolonge une session à partir de l'identifiant porté par le cookie.
    ///
    /// `Err(Forbidden)` = session inconnue ou refresh refusé par Discord ;
    /// l'appelant efface le cookie. Toute autre erreur est une panne, et ne
    /// doit PAS déconnecter l'utilisateur.
    async fn refresh(&self, session_id: Uuid) -> Result<EstablishedSession, DomainError>;

    /// Révoque la session. Idempotent : une session déjà absente n'est pas une
    /// erreur, le demandeur voulait ne plus être connecté et il ne l'est pas.
    async fn logout(&self, session_id: Uuid) -> Result<(), DomainError>;

    async fn recent_logins(&self, limit: i64) -> Result<Vec<SuccessfulLogin>, DomainError>;

    async fn purge_logins(&self, days: i32) -> Result<u64, DomainError>;
}
