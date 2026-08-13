use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;

/// Adapter sortant du domaine snapshots analytics. Tout le SQL brut des jobs
/// (baselines quotidiennes/horaires, purge de retention, liste des guilds) vit
/// derriere ce port ; le use case reste pur.
#[async_trait]
pub trait SnapshotRepository: Send + Sync {
    /// Liste les guilds connues (ordre stable par nom).
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError>;

    /// Snapshot quotidien via baseline figee : capture la baseline du "stat day"
    /// courant (ON CONFLICT DO NOTHING) puis upsert le delta dans
    /// `daily_activity`. `anchor_hour` est clampe 0..23 par l'appelant.
    async fn snapshot_daily(
        &self,
        guild_id: &str,
        track_messages: bool,
        track_voice: bool,
        anchor_hour: i64,
    ) -> Result<(), DomainError>;

    /// Snapshot horaire : upsert de `hourly_activity` pour l'heure courante.
    async fn snapshot_hourly(
        &self,
        guild_id: &str,
        track_messages: bool,
    ) -> Result<(), DomainError>;

    /// Purge `daily_activity` au dela de `retention_days`.
    async fn cleanup_daily(&self, guild_id: &str, retention_days: i32) -> Result<(), DomainError>;

    /// Purge `analytics_daily_baseline` au dela de `retention_days`.
    async fn cleanup_baseline(
        &self,
        guild_id: &str,
        retention_days: i32,
    ) -> Result<(), DomainError>;

    /// Purge `hourly_activity` au dela de `retention_days`.
    async fn cleanup_hourly(&self, guild_id: &str, retention_days: i32) -> Result<(), DomainError>;
}
