use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::confession::{
    Confession, ConfessionConfig, ConfessionReply, ConfessionReport, ReportStatus,
};
use crate::sentinel::domain::errors::DomainError;

pub struct CreateConfessionCommand {
    pub guild_id: String,
    pub author_user_id: String,
    pub content: String,
}

pub struct CreateReplyCommand {
    pub confession_id: Uuid,
    pub author_user_id: String,
    pub content: String,
    pub is_anonymous: bool,
}

pub struct CreateReportCommand {
    pub guild_id: String,
    pub confession_id: Option<Uuid>,
    pub reply_id: Option<Uuid>,
    pub reporter_user_id: String,
    pub reason: String,
}

#[async_trait]
pub trait ManageConfessionsUseCase: Send + Sync {
    /// Cree une confession (apres validation cooldown/quota/contenu).
    /// Le bot recupere le resultat puis poste sur Discord et appelle
    /// update_message_refs pour lier le message_id.
    async fn create(&self, cmd: CreateConfessionCommand) -> Result<Confession, DomainError>;

    /// Le bot enregistre les refs Discord apres avoir poste.
    async fn update_message_refs(
        &self,
        id: Uuid,
        message_id: String,
        channel_id: String,
        thread_id: Option<String>,
    ) -> Result<(), DomainError>;

    async fn edit_content(
        &self,
        id: Uuid,
        author_user_id: &str,
        new_content: String,
    ) -> Result<Confession, DomainError>;

    /// Soft delete (depuis web admin OU depuis bot slash command).
    async fn delete(
        &self,
        id: Uuid,
        deleted_by: String,
        reason: Option<String>,
    ) -> Result<Confession, DomainError>;

    async fn get(&self, id: Uuid) -> Result<Confession, DomainError>;
    async fn get_by_message_id(&self, message_id: &str) -> Result<Option<Confession>, DomainError>;
    async fn get_by_public_number(
        &self,
        guild_id: &str,
        public_number: i32,
    ) -> Result<Confession, DomainError>;
    async fn list(
        &self,
        guild_id: &str,
        limit: i64,
        include_deleted: bool,
    ) -> Result<Vec<Confession>, DomainError>;

    // ── Replies ────────────────────────────────────────────────────────

    async fn create_reply(&self, cmd: CreateReplyCommand) -> Result<ConfessionReply, DomainError>;
    async fn update_reply_message_id(
        &self,
        id: Uuid,
        message_id: String,
    ) -> Result<(), DomainError>;
    async fn delete_reply(
        &self,
        id: Uuid,
        deleted_by: String,
    ) -> Result<ConfessionReply, DomainError>;
    async fn list_replies(&self, confession_id: Uuid) -> Result<Vec<ConfessionReply>, DomainError>;

    /// Resout le `guild_id` de la confession parente d'une reply. Sert au
    /// gating RBAC web (le path `/replies/{id}` n'a pas de guild_id).
    async fn get_reply_parent_guild(&self, reply_id: Uuid) -> Result<String, DomainError>;

    // ── Reports ────────────────────────────────────────────────────────

    async fn create_report(
        &self,
        cmd: CreateReportCommand,
    ) -> Result<ConfessionReport, DomainError>;
    async fn list_reports(
        &self,
        guild_id: &str,
        status: Option<ReportStatus>,
        limit: i64,
    ) -> Result<Vec<ConfessionReport>, DomainError>;
    /// Resout le `guild_id` d'un signalement. Sert au gating RBAC web (le path
    /// `/reports/{id}/resolve` n'a pas de guild_id).
    async fn get_report_guild(&self, report_id: Uuid) -> Result<String, DomainError>;
    async fn resolve_report(
        &self,
        id: Uuid,
        status: ReportStatus,
        resolved_by: String,
    ) -> Result<(), DomainError>;

    // ── Config ─────────────────────────────────────────────────────────

    async fn get_config(&self, guild_id: &str) -> Result<ConfessionConfig, DomainError>;
    async fn save_config(&self, cfg: ConfessionConfig) -> Result<ConfessionConfig, DomainError>;
}
