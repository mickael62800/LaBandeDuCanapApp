//! Adapter du port `Fail2banStatusReader` : lit le JSON de statut fail2ban
//! expose par le cron host `/usr/local/bin/fail2ban-export.sh`.

use async_trait::async_trait;

use ops_core::domain::entities::ip_ban::{Fail2banJail, Fail2banStatus};
use sentinel_core::domain::errors::DomainError;
use ops_core::ports::outbound::host_ban_queue::Fail2banStatusReader;

const F2B_STATUS_PATH: &str = "/var/lib/sentinel/fail2ban-status.json";

#[derive(serde::Deserialize)]
struct RawJail {
    name: String,
    total_banned: i64,
    banned_ips: String,
}

#[derive(serde::Deserialize)]
struct RawStatus {
    updated_at: String,
    jails: Vec<RawJail>,
}

#[derive(Default)]
pub struct Fail2banFileReader;

impl Fail2banFileReader {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Fail2banStatusReader for Fail2banFileReader {
    async fn read_status(&self) -> Result<Option<Fail2banStatus>, DomainError> {
        let raw = match std::fs::read_to_string(F2B_STATUS_PATH) {
            Ok(s) => s,
            Err(_) => return Ok(None), // fail2ban non installe
        };

        let parsed: RawStatus = serde_json::from_str(&raw)
            .map_err(|e| DomainError::Internal(format!("parse fail2ban-status.json: {e}")))?;

        let jails = parsed
            .jails
            .into_iter()
            .map(|j| Fail2banJail {
                name: j.name,
                total_banned: j.total_banned,
                banned_ips: j
                    .banned_ips
                    .split_whitespace()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect(),
            })
            .collect();

        Ok(Some(Fail2banStatus {
            updated_at: parsed.updated_at,
            jails,
        }))
    }
}
