//! Cycle de vie d'une session web : login, refresh, logout.
//!
//! Porté depuis `sentinel-api/.../handlers/system/oauth.rs`. L'échange de
//! jetons y était resté volontairement dans le handler, « indissociable du flux
//! CSRF/cookies ». Cet argument tenait tant que l'OAuth n'était qu'une route
//! parmi cent dans une API de modération ; dans une plateforme dont c'est le
//! seul métier, il devient le métier — et un service qu'on ne peut pas tester
//! sans réseau serait le mauvais choix.
//!
//! Ce qui RESTE dans l'adaptateur HTTP : les cookies et les redirections. Ce
//! sont des préoccupations de transport, et le service n'a pas à savoir qu'il
//! sert un navigateur.

use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::identity::{SuperadminPolicy, TokenPair};
use crate::domain::entities::session::{
    NewOAuthSession, SessionTokenUpdate, SuccessfulLogin, SESSION_MAX_AGE_SECS,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_session::{
    EstablishedSession, LoginContext, ManageSessionUseCase,
};
use crate::ports::outbound::discord_identity::DiscordIdentity;
use crate::ports::outbound::login_state_store::LoginStateStore;
use crate::ports::outbound::session_repository::SessionRepository;

const STATE_TTL_SECS: u64 = 600;
/// Marge avant expiration en deçà de laquelle on rafraîchit plutôt que de
/// rendre le token courant. Sans elle, un token rendu à T-1s expirerait entre
/// la réponse et le premier appel du front.
const REFRESH_MARGIN_SECS: i64 = 60;
/// Durée de vie supposée quand Discord ne l'annonce pas (7 jours, sa valeur
/// habituelle).
const DEFAULT_EXPIRES_IN_SECS: i64 = 604_800;

pub struct ManageSessionService {
    pub sessions: Arc<dyn SessionRepository>,
    pub discord: Arc<dyn DiscordIdentity>,
    pub states: Arc<dyn LoginStateStore>,
    pub policy: SuperadminPolicy,
    /// Générateur du `state` CSRF. Injecté : l'aléa est de l'infra, et un test
    /// doit pouvoir le rendre déterministe.
    pub new_state: fn() -> String,
}

impl ManageSessionService {
    fn expires_at(tokens: &TokenPair) -> chrono::DateTime<Utc> {
        let secs = if tokens.expires_in_secs > 0 {
            tokens.expires_in_secs
        } else {
            DEFAULT_EXPIRES_IN_SECS
        };
        Utc::now() + Duration::seconds(secs)
    }
}

#[async_trait]
impl ManageSessionUseCase for ManageSessionService {
    async fn start_login(&self) -> Result<String, DomainError> {
        let state = (self.new_state)();
        self.states.put(&state, STATE_TTL_SECS).await?;
        Ok(self.discord.authorize_url(&state))
    }

    async fn complete_login(
        &self,
        code: &str,
        state: &str,
        context: LoginContext,
    ) -> Result<EstablishedSession, DomainError> {
        // Le `state` est consommé AVANT l'échange : un code rejoué avec un
        // state déjà utilisé ne doit pas atteindre Discord.
        if !self.states.take(state).await? {
            return Err(DomainError::Forbidden(
                "state OAuth invalide ou expire".into(),
            ));
        }

        let tokens = self.discord.exchange_code(code).await?;
        let user = self.discord.get_user_me(&tokens.access_token).await?;

        // Trace best-effort : un journal indisponible ne doit pas refuser un
        // login par ailleurs valide.
        if let Err(error) = self
            .sessions
            .record_login(&context.into_trace(user.id.clone(), user.username.clone()))
            .await
        {
            tracing::warn!(%error, "trace de login non enregistree");
        }

        // Pas de refresh token = pas de session persistante. Le login reussit
        // quand meme, sans « rester connecte » : comportement historique.
        let session_id = if tokens.refresh_token.is_empty() {
            None
        } else {
            let id = Uuid::new_v4();
            let created = self
                .sessions
                .create_session(&NewOAuthSession {
                    id,
                    discord_user_id: user.id.clone(),
                    username: user.username.clone(),
                    global_name: user.global_name.clone(),
                    avatar: user.avatar.clone(),
                    access_token: tokens.access_token.clone(),
                    refresh_token: tokens.refresh_token.clone(),
                    access_expires_at: Self::expires_at(&tokens),
                    expires_at: Utc::now() + Duration::seconds(SESSION_MAX_AGE_SECS),
                })
                .await;
            match created {
                Ok(()) => Some(id),
                // Base indisponible : on degrade vers un login sans
                // persistance plutot que de refuser l'entree.
                Err(error) => {
                    tracing::warn!(%error, "session non persistee -- login sans « rester connecte »");
                    None
                }
            }
        };

        Ok(EstablishedSession {
            session_id,
            access_token: tokens.access_token,
            is_superadmin: self.policy.grants(&user.id),
            discord_user_id: user.id,
            username: user.username,
            global_name: user.global_name,
            avatar: user.avatar,
        })
    }

    async fn refresh(&self, session_id: Uuid) -> Result<EstablishedSession, DomainError> {
        let session = self
            .sessions
            .find_by_id(session_id)
            .await?
            .ok_or_else(|| DomainError::Forbidden("session inconnue".into()))?;

        // Defense en profondeur : le depot Postgres filtre deja cette ligne,
        // mais toute implementation future du port doit rester fail-closed.
        if session.expires_at <= Utc::now() {
            let _ = self.sessions.delete(session_id).await;
            return Err(DomainError::Forbidden("session expiree".into()));
        }

        let established = |access_token: String| EstablishedSession {
            session_id: Some(session_id),
            access_token,
            is_superadmin: self.policy.grants(&session.discord_user_id),
            discord_user_id: session.discord_user_id.clone(),
            username: session.username.clone(),
            global_name: session.global_name.clone(),
            avatar: session.avatar.clone(),
        };

        // Token encore valide : on le rend tel quel, sans deranger Discord.
        if session.access_expires_at > Utc::now() + Duration::seconds(REFRESH_MARGIN_SECS) {
            let _ = self.sessions.touch(session_id).await;
            return Ok(established(session.access_token));
        }

        let tokens = match self.discord.refresh(&session.refresh_token).await {
            Ok(t) => t,
            Err(error) => {
                // Un refresh REFUSE invalide la session (l'utilisateur a revoque
                // l'application). Une panne reseau, non : la distinction est
                // portee par le type d'erreur, et la confondre deconnecterait
                // tout le monde a chaque hoquet de Discord.
                if matches!(error, DomainError::Forbidden(_)) {
                    let _ = self.sessions.delete(session_id).await;
                }
                return Err(error);
            }
        };

        // Discord peut faire tourner le refresh token : on garde le nouveau
        // s'il en rend un, l'ancien sinon.
        let refresh_token = if tokens.refresh_token.is_empty() {
            session.refresh_token.clone()
        } else {
            tokens.refresh_token.clone()
        };

        self.sessions
            .update_tokens(&SessionTokenUpdate {
                id: session_id,
                access_token: tokens.access_token.clone(),
                refresh_token,
                access_expires_at: Self::expires_at(&tokens),
            })
            .await?;

        Ok(established(tokens.access_token))
    }

    async fn logout(&self, session_id: Uuid) -> Result<(), DomainError> {
        self.sessions.delete(session_id).await
    }

    async fn recent_logins(&self, limit: i64) -> Result<Vec<SuccessfulLogin>, DomainError> {
        self.sessions.list_recent_logins(limit).await
    }

    async fn purge_logins(&self, days: i32) -> Result<u64, DomainError> {
        self.sessions.purge_logins_older_than(days).await
    }
}

#[cfg(test)]
#[path = "tests/manage_session_service.rs"]
mod tests;
