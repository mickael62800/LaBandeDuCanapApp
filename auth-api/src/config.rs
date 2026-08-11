//! Configuration lue au démarrage.

use auth_core::domain::entities::identity::SuperadminPolicy;

pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub bind_addr: std::net::SocketAddr,
    /// Jeton des appelants de service (nginx, sentinel-api, ops-api).
    ///
    /// Vide = les routes de service sont ouvertes. Toléré en développement
    /// seulement ; le compose l'exige.
    pub api_token: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_redirect_uri: String,
    /// Où renvoyer le navigateur après le flux OAuth.
    pub web_front_url: String,
    pub superadmins: SuperadminPolicy,
    /// `Secure` sur les cookies de session. Désactivable UNIQUEMENT pour un
    /// développement en http:// — un navigateur refuse un cookie `Secure` hors
    /// TLS, et le « rester connecté » ne marcherait jamais en local.
    pub cookie_secure: bool,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let superadmins =
            SuperadminPolicy::from_csv(&std::env::var("SUPERADMIN_USER_IDS").unwrap_or_default());
        if superadmins.is_empty() {
            // Fail-closed : on démarre quand même (les services internes en ont
            // besoin), mais aucun humain n'entrera. Le dire fort évite une
            // demi-heure à chercher pourquoi le back-office répond 403.
            tracing::warn!(
                "SUPERADMIN_USER_IDS vide — aucun compte Discord ne pourra entrer dans le back-office"
            );
        }

        Self {
            database_url: std::env::var("AUTH_DATABASE_URL")
                .expect("AUTH_DATABASE_URL manquante (ex: postgres://user:pass@host/auth)"),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
            bind_addr: std::env::var("AUTH_API_BIND_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8096".into())
                .parse()
                .expect("AUTH_API_BIND_ADDR invalide"),
            api_token: std::env::var("AUTH_API_TOKEN").unwrap_or_default(),
            discord_client_id: std::env::var("DISCORD_CLIENT_ID").unwrap_or_default(),
            discord_client_secret: std::env::var("DISCORD_CLIENT_SECRET").unwrap_or_default(),
            discord_redirect_uri: std::env::var("DISCORD_REDIRECT_URI").unwrap_or_default(),
            web_front_url: std::env::var("WEB_FRONT_URL").unwrap_or_default(),
            superadmins,
            cookie_secure: std::env::var("AUTH_COOKIE_SECURE")
                .map(|v| v != "false")
                .unwrap_or(true),
        }
    }
}
