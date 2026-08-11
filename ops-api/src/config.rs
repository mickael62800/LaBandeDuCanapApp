//! Configuration lue au demarrage.

use std::net::SocketAddr;

pub struct AppConfig {
    pub bind_addr: SocketAddr,
    /// Base de Sentinel, via un role restreint (cf. doc du crate).
    pub database_url: String,
    /// Jeton injecte par nginx sur `/ops-api/`. Le navigateur ne le voit jamais.
    pub api_token: String,
    /// Protection optionnelle de `/metrics`. Vide = ouvert sur le reseau
    /// interne, ou Prometheus scrape — comme les trois autres API.
    pub metrics_token: String,
    /// Redis, pour la deduplication des alertes (cle a TTL = cooldown).
    pub redis_url: String,
    /// `docker-agent`, seul service a monter le socket de l'hote.
    pub docker_agent_url: String,
    pub docker_agent_token: String,
    pub rate_limit_per_sec: u64,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let bind_addr = std::env::var("OPS_API_BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:3200".into())
            .parse()
            .map_err(|_| "OPS_API_BIND_ADDR invalide".to_owned())?;

        // Requis, sans valeur par defaut : demarrer sans jeton exposerait
        // l'administration de la machine a tout le reseau interne.
        let api_token = std::env::var("OPS_API_TOKEN")
            .ok()
            .filter(|value| value.trim().len() >= 16)
            .ok_or("OPS_API_TOKEN manquant ou trop court (16 caracteres minimum)")?;

        Ok(Self {
            bind_addr,
            database_url: std::env::var("OPS_DATABASE_URL")
                .map_err(|_| "OPS_DATABASE_URL manquante".to_owned())?,
            api_token,
            metrics_token: std::env::var("OPS_METRICS_TOKEN").unwrap_or_default(),
            redis_url: std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis:6379".into()),
            docker_agent_url: std::env::var("DOCKER_AGENT_URL")
                .unwrap_or_else(|_| "http://docker-agent:8095".into()),
            docker_agent_token: std::env::var("DOCKER_AGENT_TOKEN").unwrap_or_default(),
            rate_limit_per_sec: std::env::var("OPS_API_RATE_LIMIT_PER_SEC")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(20),
        })
    }

    #[cfg(test)]
    pub fn dummy() -> Self {
        Self {
            bind_addr: "127.0.0.1:3200".parse().unwrap(),
            database_url: "postgres://localhost/test".into(),
            api_token: "test-token-suffisamment-long".into(),
            metrics_token: String::new(),
            redis_url: "redis://localhost".into(),
            docker_agent_url: "http://localhost:8095".into(),
            docker_agent_token: "t".into(),
            rate_limit_per_sec: 20,
        }
    }
}
