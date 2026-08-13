use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::sentinel::domain::entities::community::confession::{
    Confession, ConfessionConfig, ConfessionReply, ConfessionReport, ReportStatus,
};
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ConfessionRepository: Send + Sync {
    /// Atomique : incremente le compteur de la guild et retourne le nouveau
    /// numero (1, 2, 3, ...). Premier appel pour une guild = 1.
    async fn next_public_number(&self, guild_id: &str) -> Result<i32, DomainError>;

    async fn create_confession(&self, c: &Confession) -> Result<(), DomainError>;
    async fn update_confession_message_refs(
        &self,
        id: Uuid,
        message_id: &str,
        channel_id: &str,
        thread_id: Option<&str>,
    ) -> Result<(), DomainError>;
    async fn edit_confession_content(&self, id: Uuid, content: &str) -> Result<(), DomainError>;
    async fn soft_delete_confession(
        &self,
        id: Uuid,
        deleted_by: &str,
        reason: Option<&str>,
    ) -> Result<(), DomainError>;
    async fn get_confession(&self, id: Uuid) -> Result<Option<Confession>, DomainError>;
    async fn get_by_message_id(&self, message_id: &str) -> Result<Option<Confession>, DomainError>;
    async fn get_by_public_number(
        &self,
        guild_id: &str,
        public_number: i32,
    ) -> Result<Option<Confession>, DomainError>;
    async fn list_by_guild(
        &self,
        guild_id: &str,
        limit: i64,
        include_deleted: bool,
    ) -> Result<Vec<Confession>, DomainError>;

    /// Cooldown : combien de confessions cet user a poste depuis `since`.
    async fn count_recent_by_author(
        &self,
        guild_id: &str,
        author_user_id: &str,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError>;

    // ── Replies ────────────────────────────────────────────────────────

    /// Numero suivant pour une reply de cette confession (1, 2, 3...).
    async fn next_reply_public_number(&self, confession_id: Uuid) -> Result<i32, DomainError>;
    async fn create_reply(&self, r: &ConfessionReply) -> Result<(), DomainError>;
    async fn update_reply_message_id(&self, id: Uuid, message_id: &str) -> Result<(), DomainError>;
    async fn soft_delete_reply(&self, id: Uuid, deleted_by: &str) -> Result<(), DomainError>;
    async fn list_replies(&self, confession_id: Uuid) -> Result<Vec<ConfessionReply>, DomainError>;
    async fn get_reply(&self, id: Uuid) -> Result<Option<ConfessionReply>, DomainError>;

    // ── Reports ────────────────────────────────────────────────────────

    async fn create_report(&self, r: &ConfessionReport) -> Result<(), DomainError>;
    async fn get_report(&self, id: Uuid) -> Result<Option<ConfessionReport>, DomainError>;
    async fn list_reports(
        &self,
        guild_id: &str,
        status: Option<ReportStatus>,
        limit: i64,
    ) -> Result<Vec<ConfessionReport>, DomainError>;
    async fn resolve_report(
        &self,
        id: Uuid,
        status: ReportStatus,
        resolved_by: &str,
    ) -> Result<(), DomainError>;

    // ── Config ─────────────────────────────────────────────────────────

    async fn get_config(&self, guild_id: &str) -> Result<Option<ConfessionConfig>, DomainError>;
    async fn upsert_config(&self, c: &ConfessionConfig) -> Result<(), DomainError>;
}
