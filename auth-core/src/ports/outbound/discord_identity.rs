//! Appels sortants vers Discord : échange de jetons et lecture d'identité.
//!
//! L'échange `/oauth2/token` était resté dans le handler HTTP de
//! `sentinel-api`, jugé indissociable du flux CSRF/cookies. Il devient ici un
//! port : dans une plateforme dont c'est le seul métier, cet appel EST le
//! métier, et le laisser dans l'adaptateur rendrait le service intestable sans
//! réseau.

use async_trait::async_trait;

use crate::domain::entities::identity::{DiscordUser, TokenPair};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait DiscordIdentity: Send + Sync {
    /// URL d'autorisation Discord, `state` CSRF inclus.
    fn authorize_url(&self, state: &str) -> String;

    /// Échange un code d'autorisation contre un couple de jetons.
    async fn exchange_code(&self, code: &str) -> Result<TokenPair, DomainError>;

    /// Rejoue un refresh token pour prolonger la session.
    async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, DomainError>;

    /// `GET /users/@me` — l'identité derrière un access token.
    async fn get_user_me(&self, access_token: &str) -> Result<DiscordUser, DomainError>;
}
