use std::time::Duration;

use crate::http::HttpJobClient;

#[derive(Clone)]
pub struct DomainConfig {
    pub client: HttpJobClient,
}

pub struct Config {
    pub atrium: Option<DomainConfig>,
    pub nexus: Option<DomainConfig>,
    pub ops: Option<DomainConfig>,
    pub sentinel: Option<DomainConfig>,
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

fn domain(prefix: &str) -> Result<Option<DomainConfig>, String> {
    let enabled_name = format!("SCHEDULER_{prefix}_ENABLED");
    let enabled = std::env::var(&enabled_name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);
    if !enabled {
        return Ok(None);
    }

    let url_name = format!("{prefix}_API_URL");
    let token_name = format!("{prefix}_API_TOKEN");
    let api_url = required(&url_name)?;
    let token = required(&token_name)?;
    Ok(Some(DomainConfig {
        client: HttpJobClient::new(api_url, token, Duration::from_secs(30)),
    }))
}

fn required(name: &str) -> Result<String, String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} est requis quand le domaine est active"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_domain_needs_no_secret() {
        std::env::remove_var("SCHEDULER_TEST_DISABLED_ENABLED");
        assert!(domain("TEST_DISABLED").unwrap().is_none());
    }
}
