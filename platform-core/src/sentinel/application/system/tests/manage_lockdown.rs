use super::*;
use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::system::manage_lockdown::{StartLockdownCommand, EndLockdownCommand};

#[tokio::test]
async fn start_lockdown_valid() {
    let cmd = StartLockdownCommand {
        guild_id: "guild123".to_string(),
        reason: "Raid detected".to_string(),
        initiated_by: "mod123".to_string(),
    };
    assert!(!cmd.guild_id.is_empty());
    assert!(!cmd.reason.is_empty());
}

#[tokio::test]
async fn end_lockdown_valid() {
    let cmd = EndLockdownCommand {
        guild_id: "guild123".to_string(),
        ended_by: "mod123".to_string(),
    };
    assert!(!cmd.guild_id.is_empty());
}
