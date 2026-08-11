/// Configuration du gateway chargee depuis les variables d'environnement.
pub struct Config {
    pub host: String,
    pub port: u16,
    pub redis_url: String,
    pub api_key: String,
    pub redis_channel: String,
    pub allowed_origins: String,
    pub max_connections: usize,
    pub api_url: String,
    pub broadcast_capacity: usize,
    pub redis_reconnect_delay_secs: u64,
    pub redis_reconnect_max_delay_secs: u64,
    pub cors_max_age_secs: u64,
    pub shutdown_timeout_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3001),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            api_key: std::env::var("SENTINEL_API_KEY")
                .or_else(|_| std::env::var("API_KEY"))
                .unwrap_or_default(),
            redis_channel: std::env::var("REDIS_CHANNEL")
                .unwrap_or_else(|_| "sentinel:events".to_string()),
            allowed_origins: std::env::var("ALLOWED_ORIGINS").unwrap_or_default(),
            max_connections: std::env::var("MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            api_url: std::env::var("API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            broadcast_capacity: std::env::var("BROADCAST_CHANNEL_CAPACITY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(512),
            redis_reconnect_delay_secs: std::env::var("REDIS_RECONNECT_DELAY_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
            redis_reconnect_max_delay_secs: std::env::var("REDIS_RECONNECT_MAX_DELAY_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            cors_max_age_secs: std::env::var("CORS_MAX_AGE_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            shutdown_timeout_secs: std::env::var("SHUTDOWN_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
