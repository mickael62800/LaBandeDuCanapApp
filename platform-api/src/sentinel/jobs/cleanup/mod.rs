pub mod cleanup_old_data;
pub mod vacuum_tables;

#[derive(Clone)]
pub struct CleanupConfig {
    pub voice_sessions_retention_days: i64,
    pub logs_retention_days: i64,
    pub closed_tickets_retention_days: i64,
}
