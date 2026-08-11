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

    /// Purge distante du journal des logins. Renvoie le nombre supprime, ou une
    /// raison d'echec : la purge de l'identite est hors de la transaction locale
    /// (bases distinctes) et ne doit pas faire echouer le nettoyage des autres
    /// tables, mais son echec doit etre VISIBLE (et non masque en 0).
    pub async fn purge(&self, days: i64) -> Result<u64, String> {
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
            Ok(r) if r.status().is_success() => r
                .json::<Purged>()
                .await
                .map(|p| p.deleted)
                .map_err(|error| {
                    tracing::warn!(%error, "reponse de purge auth-api illisible");
                    "reponse auth-api illisible".to_owned()
                }),
            Ok(r) => {
                let status = r.status();
                tracing::warn!(%status, "purge des logins refusee par auth-api");
                Err(format!("auth-api a refuse la purge ({status})"))
            }
            Err(error) => {
                tracing::warn!(%error, "purge des logins impossible");
                Err("auth-api injoignable".to_owned())
            }
        }
    }
}
