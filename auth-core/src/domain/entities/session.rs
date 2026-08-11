//! Session web persistante (refresh token côté serveur) et trace de login.
//!
//! Repris tel quel de `sentinel-core/domain/entities/system/oauth.rs` : le
//! modèle était déjà correct, il changeait seulement de propriétaire.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trace best-effort d'un login OAuth reussi (journal `successful_logins`).
///
/// Écrite par le flux d'authentification, lue par l'écran « Sécurité de
/// l'hôte » de l'exploitation. Depuis l'extraction, `ops-api` ne la lit plus en
/// SQL — elle appartient à la base de l'identité, et l'exploitation la demande
/// à `auth-api`. Même relation qu'avec `docker-agent` : le propriétaire de la
/// donnée l'expose, il ne la partage pas par la base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginTrace {
    pub discord_user_id: String,
    pub username: String,
    pub client_ip: String,
    pub user_agent: String,
}

/// Login réussi tel que relu pour l'écran de sécurité.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessfulLogin {
    pub discord_user_id: String,
    pub username: String,
    pub client_ip: String,
    pub user_agent: String,
    pub logged_at: DateTime<Utc>,
}

/// Donnees de creation d'une session web persistante.
#[derive(Debug, Clone)]
pub struct NewOAuthSession {
    pub id: Uuid,
    pub discord_user_id: String,
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
}

/// Session web persistante telle que relue depuis le stockage.
#[derive(Debug, Clone)]
pub struct OAuthSession {
    pub id: Uuid,
    pub discord_user_id: String,
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
}

/// Mise a jour des tokens d'une session apres un refresh Discord.
#[derive(Debug, Clone)]
pub struct SessionTokenUpdate {
    pub id: Uuid,
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
}
