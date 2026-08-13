use std::time::Duration;

use crate::http::HttpJobClient;

#[derive(Clone)]
pub struct DomainConfig {
    pub client: HttpJobClient,
}

pub struct Config {
    pub atrium: DomainConfig,
    pub nexus: DomainConfig,
    pub ops: DomainConfig,
    pub sentinel: DomainConfig,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            atrium: domain("ATRIUM")?,
            nexus: domain("NEXUS")?,
            ops: domain("OPS")?,
            sentinel: domain("SENTINEL")?,
        })
    }
}

fn domain(prefix: &str) -> Result<DomainConfig, String> {
    let url_name = format!("{prefix}_API_URL");
    let token_name = format!("{prefix}_API_TOKEN");
    let api_url = required(&url_name)?;
    let token = required(&token_name)?;
    Ok(DomainConfig {
        client: HttpJobClient::new(api_url, token, Duration::from_secs(30)),
    })
}

fn required(name: &str) -> Result<String, String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} est requis"))
}
