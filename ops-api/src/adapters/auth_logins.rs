//! Client du journal de logins servi par `auth-api`.
//!
//! # Pourquoi un appel reseau plutot qu'un SELECT
//!
//! `successful_logins` etait dans `discord_sentinel`, et l'exploitation la
//! lisait en SQL. La table appartient desormais a la base de l'identite —
//! Postgres ne sait pas requeter entre bases logiques, et surtout : le
//! proprietaire d'une donnee l'expose, il ne la partage pas par la base. Meme
//! relation qu'avec `docker-agent` pour le daemon Docker.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use ops_core::domain::entities::security_audit::SuccessfulLogin;
use ops_core::domain::errors::DomainError;

#[derive(Deserialize)]
struct AuthLogin {
    discord_user_id: String,
    username: String,
    client_ip: String,
    user_agent: String,
    logged_at: DateTime<Utc>,
}

pub struct AuthLoginsClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl AuthLoginsClient {
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            base_url,
            token,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    pub async fn recent(&self, limit: i64) -> Result<Vec<SuccessfulLogin>, DomainError> {
        let response = self
            .client
            .get(self.url("/security/last-logins"))
            .query(&[("limit", limit)])
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| {
                tracing::warn!(%error, "auth-api injoignable");
                DomainError::Internal("auth-api injoignable".into())
            })?;

        if !response.status().is_success() {
            let status = response.status();
            tracing::warn!(%status, "auth-api a refuse la lecture des logins");
            return Err(DomainError::Internal("auth-api indisponible".into()));
        }

        let rows: Vec<AuthLogin> = response.json().await.map_err(|error| {
            tracing::warn!(%error, "reponse auth-api illisible");
            DomainError::Internal("reponse auth-api illisible".into())
        })?;

        Ok(rows
            .into_iter()
            .map(|r| SuccessfulLogin {
                timestamp: r.logged_at,
                discord_user_id: r.discord_user_id,
                // L'identite sert des chaines toujours presentes ; le domaine
                // d'exploitation les modelise en `Option` depuis l'epoque ou
                // elles venaient d'un SQL avec des NULL. On normalise ici :
                // une chaine vide vaut « pas renseigne ».
                username: Some(r.username).filter(|s| !s.is_empty()),
                client_ip: Some(r.client_ip).filter(|s| !s.is_empty()),
                user_agent: Some(r.user_agent).filter(|s| !s.is_empty()),
            })
            .collect())
    }

    /// Purge best-effort, alignee sur le reste de `cleanup` : une panne de
    /// l'identite ne doit pas faire echouer le nettoyage des autres tables.
    pub async fn purge(&self, days: i64) -> u64 {
        let response = self
            .client
            .post(self.url("/security/purge-logins"))
            .query(&[("days", days)])
            .bearer_auth(&self.token)
            .send()
            .await;

        #[derive(Deserialize)]
        struct Purged {
            deleted: u64,
        }

        match response {
            Ok(r) if r.status().is_success() => {
                r.json::<Purged>().await.map(|p| p.deleted).unwrap_or(0)
            }
            Ok(r) => {
                tracing::warn!(status = %r.status(), "purge des logins refusee par auth-api");
                0
            }
            Err(error) => {
                tracing::warn!(%error, "purge des logins impossible");
                0
            }
        }
    }
}
