//! Audit log dedie au game portal (qui a fait quoi sur quel serveur).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Actions recensees. Etendre selon les besoins futurs (backup, restore...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameAuditAction {
    Create,
    Start,
    Stop,
    Restart,
    Delete,
    ConfigUpdate,
    CommandRcon,
    IdleShutdown,
    CrashDetected,
    AutoRestart,
    BackupCreate,
    BackupRestore,
    IpReveal,
    /// Ouverture programmee (mode « Préparation ») ou ajustement de l'heure de
    /// révélation.
    Schedule,
}

impl GameAuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
            Self::Delete => "delete",
            Self::ConfigUpdate => "config_update",
            Self::CommandRcon => "command_rcon",
            Self::IdleShutdown => "idle_shutdown",
            Self::CrashDetected => "crash_detected",
            Self::AutoRestart => "auto_restart",
            Self::BackupCreate => "backup_create",
            Self::BackupRestore => "backup_restore",
            Self::IpReveal => "ip_reveal",
            Self::Schedule => "schedule",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameAuditEntry {
    pub id: Uuid,
    pub server_id: Option<Uuid>,
    pub guild_id: String,
    pub actor_user_id: Option<String>,
    pub action: String,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_audit_action_as_str() {
        assert_eq!(GameAuditAction::Create.as_str(), "create");
        assert_eq!(GameAuditAction::Start.as_str(), "start");
        assert_eq!(GameAuditAction::Stop.as_str(), "stop");
        assert_eq!(GameAuditAction::Restart.as_str(), "restart");
        assert_eq!(GameAuditAction::Delete.as_str(), "delete");
        assert_eq!(GameAuditAction::ConfigUpdate.as_str(), "config_update");
        assert_eq!(GameAuditAction::CommandRcon.as_str(), "command_rcon");
        assert_eq!(GameAuditAction::IdleShutdown.as_str(), "idle_shutdown");
        assert_eq!(GameAuditAction::CrashDetected.as_str(), "crash_detected");
        assert_eq!(GameAuditAction::AutoRestart.as_str(), "auto_restart");
        assert_eq!(GameAuditAction::BackupCreate.as_str(), "backup_create");
        assert_eq!(GameAuditAction::BackupRestore.as_str(), "backup_restore");
        assert_eq!(GameAuditAction::IpReveal.as_str(), "ip_reveal");
    }
}
