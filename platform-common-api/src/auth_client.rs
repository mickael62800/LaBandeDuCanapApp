//! Client de l'API d'identité, partagé par les APIs qui doivent savoir **qui**
//! appelle.
//!
//! # Pourquoi ce client existe
//!
//! L'identité vivait dans `sentinel-api`, qui devenait de fait une dépendance
//! d'exécution des autres plateformes. Elle a été extraite en `auth-api`. Ce
//! client est ce qui reste du côté des consommateurs : un appel HTTP à `/access`
//! qui rend un verdict, à la place d'un middleware qui interrogeait Discord et
//! consultait une liste locale.
//!
//! Il ne contient AUCUNE règle : la décision appartient à `auth-api`. Un client
//! qui rejouerait la règle localement recréerait exactement la divergence qu'on
//! vient de supprimer.

use std::time::Duration;

/// Verdict rendu par l'API d'identité.
///
/// `Unavailable` est délibérément distinct de `Denied` : le premier doit
/// produire un **503**, le second un **403**. Les confondre ferait passer une
/// panne réseau pour une révocation de droits — l'utilisateur croirait avoir
/// perdu ses accès, et l'exploitant chercherait au mauvais endroit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessOutcome {
    /// Identité résolue et autorisée. Porte le `discord_user_id`, pour que
    /// l'appelant puisse attribuer une action à son auteur.
    Granted(String),
    /// Identité résolue, mais hors de la liste des comptes autorisés.
    Denied,
    /// Aucun jeton exploitable dans la requête.
    Unauthenticated,
    /// Impossible de trancher (identité injoignable, jeton de service refusé).
    Unavailable,
}

pub struct AuthClient {
    client: reqwest::Client,
    base_url: String,
    /// Jeton de service. Vide = développement local sans `auth-api` protégée.
    token: String,
}

impl AuthClient {
    /// `base_url` : `http://auth-api:8096` en compose.
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                // Court : cet appel est sur le chemin de CHAQUE requête
                // authentifiée. Un timeout généreux transformerait une panne
                // d'identité en gel du back-office entier.
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
            base_url,
            token,
        }
    }

    /// Résout un access token Discord en verdict d'accès.
    pub async fn resolve(&self, discord_token: &str) -> AccessOutcome {
        if discord_token.is_empty() {
            return AccessOutcome::Unauthenticated;
        }

        let request = self
            .client
            .get(format!("{}/access", self.base_url.trim_end_matches('/')))
            .header("x-discord-token", discord_token);
        let request = if self.token.is_empty() {
            request
        } else {
            request.bearer_auth(&self.token)
        };

        let response = match request.send().await {
            Ok(r) => r,
            Err(error) => {
                tracing::warn!(%error, "auth-api injoignable");
                return AccessOutcome::Unavailable;
            }
        };

        match response.status().as_u16() {
            200 => {
                let user_id = response
                    .headers()
                    .get("x-auth-user-id")
                    .and_then(|v| v.to_str().ok())
                    .filter(|id| !id.is_empty());
                match user_id {
                    Some(id) => AccessOutcome::Granted(id.to_string()),
                    // Un 200 sans identite est une incoherence de l'amont, pas
                    // une autorisation. `Granted("")` remontait jusqu'aux
                    // extensions de requete et signait les traces d'audit des
                    // actions les plus sensibles (factory reset, restore,
                    // `deleted_by`, `granted_by`) avec un auteur vide.
                    None => {
                        tracing::error!(
                            "auth-api a autorise sans en-tete x-auth-user-id — verdict ignore"
                        );
                        AccessOutcome::Unavailable
                    }
                }
            }
            403 => AccessOutcome::Denied,
            // 401 sur la sous-requête peut vouloir dire deux choses : pas de
            // jeton utilisateur, ou jeton de SERVICE refusé. Le second est une
            // erreur de configuration de notre côté, pas un refus légitime —
            // on le dit fort plutôt que de le maquiller en 401 utilisateur.
            401 if self.token.is_empty() => AccessOutcome::Unauthenticated,
            401 => {
                tracing::error!(
                    "auth-api a refuse notre jeton de service — verifier AUTH_API_TOKEN"
                );
                AccessOutcome::Unavailable
            }
            status => {
                tracing::warn!(status, "auth-api a repondu en erreur");
                AccessOutcome::Unavailable
            }
        }
    }
}
