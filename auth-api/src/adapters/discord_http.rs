//! Client Discord : échange de jetons OAuth2 et lecture d'identité.

use async_trait::async_trait;
use reqwest::{header, Client, StatusCode};
use serde::Deserialize;

use auth_core::domain::entities::identity::{DiscordUser, TokenPair};
use auth_core::domain::errors::DomainError;
use auth_core::ports::outbound::discord_identity::DiscordIdentity;

const AUTHORIZE_URL: &str = "https://discord.com/api/oauth2/authorize";
const TOKEN_URL: &str = "https://discord.com/api/v10/oauth2/token";
const ME_URL: &str = "https://discord.com/api/v10/users/@me";
const SCOPES: &str = "identify guilds";
/// Repli quand Discord n'annonce pas de duree (7 jours, sa valeur habituelle).
const DEFAULT_EXPIRES_IN: i64 = 604_800;

pub struct HttpDiscordIdentity {
    client: Client,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: Option<i64>,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct MeResponse {
    id: String,
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
}

/// Encode strict selon RFC 3986 (unreserved = alphanumeric + - . _ ~).
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl HttpDiscordIdentity {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            client_id,
            client_secret,
            redirect_uri,
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.client_id.is_empty()
            && !self.client_secret.is_empty()
            && !self.redirect_uri.is_empty()
    }

    /// Facteur commun aux deux grants. La distinction qui compte est dans le
    /// mapping d'erreur : un 4xx de Discord signifie « ce jeton ne vaut plus
    /// rien » (`Forbidden` → la session est invalidée), un échec réseau ou un
    /// 5xx signifie « on ne sait pas » (`Internal` → on ne déconnecte pas).
    async fn token_request(&self, form: &[(&str, &str)]) -> Result<TokenPair, DomainError> {
        let response = self
            .client
            .post(TOKEN_URL)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(form)
            .send()
            .await
            .map_err(|error| {
                tracing::error!(%error, "appel /oauth2/token impossible");
                DomainError::Internal("Discord injoignable".into())
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::warn!(%status, %body, "Discord a refuse l'echange de jetons");
            return Err(if status.is_client_error() {
                DomainError::Forbidden("jeton refuse par Discord".into())
            } else {
                DomainError::Internal("Discord indisponible".into())
            });
        }

        let token: TokenResponse = response.json().await.map_err(|error| {
            tracing::error!(%error, "reponse /oauth2/token illisible");
            DomainError::Internal("reponse Discord illisible".into())
        })?;

        Ok(TokenPair {
            access_token: token.access_token,
            // Chaîne vide plutôt qu'`Option` : le cœur y lit « pas de
            // persistance possible », un cas normal et non une anomalie.
            refresh_token: token.refresh_token.unwrap_or_default(),
            expires_in_secs: token.expires_in.unwrap_or(DEFAULT_EXPIRES_IN),
        })
    }
}

#[async_trait]
impl DiscordIdentity for HttpDiscordIdentity {
    fn authorize_url(&self, state: &str) -> String {
        // PAS de `prompt=none` : Discord refuse alors silencieusement avec
        // ?error=login_required quand la session navigateur a expire, ce qui
        // piege l'utilisateur dans une boucle /login -> Discord -> /login.
        // Le defaut (re-auth silencieuse si deja autorise, consentement sinon)
        // est exactement le comportement voulu.
        format!(
            "{AUTHORIZE_URL}?response_type=code&client_id={}&scope={}&redirect_uri={}&state={}",
            percent_encode(&self.client_id),
            percent_encode(SCOPES),
            percent_encode(&self.redirect_uri),
            percent_encode(state),
        )
    }

    async fn exchange_code(&self, code: &str) -> Result<TokenPair, DomainError> {
        self.token_request(&[
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.redirect_uri.as_str()),
        ])
        .await
    }

    async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, DomainError> {
        self.token_request(&[
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .await
    }

    async fn get_user_me(&self, access_token: &str) -> Result<DiscordUser, DomainError> {
        let response = self
            .client
            .get(ME_URL)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| {
                tracing::warn!(%error, "appel /users/@me impossible");
                DomainError::Internal("Discord injoignable".into())
            })?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(DomainError::Forbidden("jeton Discord invalide".into()));
        }
        if !status.is_success() {
            tracing::warn!(%status, "/users/@me a repondu en erreur");
            return Err(DomainError::Internal("Discord indisponible".into()));
        }

        let me: MeResponse = response.json().await.map_err(|error| {
            tracing::warn!(%error, "reponse /users/@me illisible");
            DomainError::Internal("reponse Discord illisible".into())
        })?;

        Ok(DiscordUser {
            id: me.id,
            username: me.username,
            global_name: me.global_name,
            avatar: me.avatar,
        })
    }
}
