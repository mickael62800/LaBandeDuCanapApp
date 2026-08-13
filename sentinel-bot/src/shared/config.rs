/// Configuration de base commune a tous les bots.
/// Les bots etendent cette struct avec leurs champs specifiques.
#[derive(Clone)]
pub struct BaseConfig {
    pub discord_token: String,
    pub api_base_url: String,
    pub api_key: String,
}

impl BaseConfig {
    /// Charge la config de base depuis les variables d'environnement.
    /// `token_var` est le nom de la variable pour le token Discord
    /// (ex: "SENTINEL_DISCORD_TOKEN", "DISCORD_TOKEN").
    pub fn from_env(token_var: &str) -> Self {
        Self {
            discord_token: std::env::var(token_var)
                .unwrap_or_else(|_| panic!("{token_var} manquant dans .env")),
            api_base_url: std::env::var("API_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            api_key: std::env::var("SENTINEL_API_KEY").unwrap_or_default(),
        }
    }
}

/// Trait que chaque Config de bot doit implementer pour acceder aux champs de base.
pub trait BotConfig {
    fn base(&self) -> &BaseConfig;

    fn discord_token(&self) -> &str {
        &self.base().discord_token
    }

    fn api_base_url(&self) -> &str {
        &self.base().api_base_url
    }

    fn api_key(&self) -> &str {
        &self.base().api_key
    }
}

// ── Config Helpers ──

/// Charge une variable d'environnement avec un fallback par defaut.
/// Utile pour les champs numeriques, booleens, etc.
///
/// ```ignore
/// let max: usize = load_env("MAX_SIZE", 100);
/// let enabled: bool = load_env("FEATURE_ENABLED", false);
/// ```
pub fn load_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Charge une variable d'environnement optionnelle.
/// Retourne `None` si absente ou non-parseable.
pub fn load_env_optional<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}

/// Charge une variable d'environnement string avec un fallback.
pub fn load_env_string(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Charge une variable d'environnement booleenne (accepte "true"/"1"/"yes",
/// insensible a la casse — sémantique de vérité unique du core).
pub fn load_env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => {
            platform_core::sentinel::domain::entities::system::config_parsers::parse_bool_str(&v)
        }
        Err(_) => default,
    }
}
