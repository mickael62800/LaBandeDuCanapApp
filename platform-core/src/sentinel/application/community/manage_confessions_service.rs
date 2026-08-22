use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::sentinel::domain::entities::community::confession::{
    Confession, ConfessionConfig, ConfessionReply, ConfessionReport, ReportStatus,
};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_confessions::{
    CreateConfessionCommand, CreateReplyCommand, CreateReportCommand, ManageConfessionsUseCase,
};
use crate::sentinel::ports::outbound::community::confession_repository::ConfessionRepository;

pub struct ManageConfessionsService {
    repo: Arc<dyn ConfessionRepository>,
}

impl ManageConfessionsService {
    pub fn new(repo: Arc<dyn ConfessionRepository>) -> Self {
        Self { repo }
    }

    async fn config_or_default(&self, guild_id: &str) -> Result<ConfessionConfig, DomainError> {
        match self.repo.get_config(guild_id).await? {
            Some(c) => Ok(c),
            None => Ok(ConfessionConfig::defaults(guild_id.to_string())),
        }
    }
}

#[async_trait]
impl ManageConfessionsUseCase for ManageConfessionsService {
    async fn create(&self, cmd: CreateConfessionCommand) -> Result<Confession, DomainError> {
        let cfg = self.config_or_default(&cmd.guild_id).await?;
        if !cfg.enabled {
            return Err(DomainError::ValidationError(
                "Systeme confessions desactive".into(),
            ));
        }
        if cfg.is_user_banned(&cmd.author_user_id) {
            return Err(DomainError::Forbidden(
                "Tu n'es plus autorise a poster".into(),
            ));
        }
        cfg.validate_content(&cmd.content)
            .map_err(DomainError::ValidationError)?;

        // Cooldown (sur la derniere confession)
        let since_cd = Utc::now() - Duration::seconds(cfg.cooldown_secs as i64);
        let recent = self
            .repo
            .count_recent_by_author(&cmd.guild_id, &cmd.author_user_id, since_cd)
            .await?;
        if recent > 0 {
            return Err(DomainError::ValidationError(format!(
                "Tu dois attendre {} secondes entre 2 confessions",
                cfg.cooldown_secs
            )));
        }
        // Quota sur fenetre glissante (config `quota_window_hours`, defaut 24h).
        let window_hours = cfg.quota_window_hours.max(1) as i64;
        let since_day = Utc::now() - Duration::hours(window_hours);
        let day_count = self
            .repo
            .count_recent_by_author(&cmd.guild_id, &cmd.author_user_id, since_day)
            .await?;
        if day_count >= cfg.max_per_day as i64 {
            return Err(DomainError::ValidationError(format!(
                "Quota atteint ({} confessions max par jour)",
                cfg.max_per_day
            )));
        }

        let public_number = self.repo.next_public_number(&cmd.guild_id).await?;
        let confession = Confession {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            public_number,
            author_user_id: cmd.author_user_id,
            content: cmd.content.trim().to_string(),
            message_id: None,
            channel_id: None,
            thread_id: None,
            deleted_at: None,
            deleted_by: None,
            deleted_reason: None,
            edited_at: None,
            created_at: Utc::now(),
        };
        self.repo.create_confession(&confession).await?;
        Ok(confession)
    }

    async fn update_message_refs(
        &self,
        id: Uuid,
        message_id: String,
        channel_id: String,
        thread_id: Option<String>,
    ) -> Result<(), DomainError> {
        self.repo
            .update_confession_message_refs(id, &message_id, &channel_id, thread_id.as_deref())
            .await
    }

    async fn edit_content(
        &self,
        id: Uuid,
        author_user_id: &str,
        new_content: String,
    ) -> Result<Confession, DomainError> {
        let mut c = self
            .repo
            .get_confession(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Confession {} introuvable", id)))?;
        if c.author_user_id != author_user_id {
            return Err(DomainError::Forbidden(
                "Cette confession n'est pas la tienne".into(),
            ));
        }
        if c.deleted_at.is_some() {
            return Err(DomainError::ValidationError(
                "Confession supprimee, edit impossible".into(),
            ));
        }
        let cfg = self.config_or_default(&c.guild_id).await?;
        cfg.validate_content(&new_content)
            .map_err(DomainError::ValidationError)?;
        let trimmed = new_content.trim().to_string();
        self.repo.edit_confession_content(id, &trimmed).await?;
        c.content = trimmed;
        c.edited_at = Some(Utc::now());
        Ok(c)
    }

    async fn delete(
        &self,
        id: Uuid,
        deleted_by: String,
        reason: Option<String>,
    ) -> Result<Confession, DomainError> {
        let mut c = self
            .repo
            .get_confession(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Confession {} introuvable", id)))?;
        if c.deleted_at.is_some() {
            return Ok(c); // Idempotent
        }
        self.repo
            .soft_delete_confession(id, &deleted_by, reason.as_deref())
            .await?;
        c.deleted_at = Some(Utc::now());
        c.deleted_by = Some(deleted_by);
        c.deleted_reason = reason;
        Ok(c)
    }

    async fn get(&self, id: Uuid) -> Result<Confession, DomainError> {
        self.repo
            .get_confession(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Confession {} introuvable", id)))
    }

    async fn get_by_message_id(&self, message_id: &str) -> Result<Option<Confession>, DomainError> {
        self.repo.get_by_message_id(message_id).await
    }

    async fn get_by_public_number(
        &self,
        guild_id: &str,
        public_number: i32,
    ) -> Result<Confession, DomainError> {
        self.repo
            .get_by_public_number(guild_id, public_number)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Confession #{} introuvable", public_number))
            })
    }

    async fn list(
        &self,
        guild_id: &str,
        limit: i64,
        include_deleted: bool,
    ) -> Result<Vec<Confession>, DomainError> {
        self.repo
            .list_by_guild(guild_id, limit, include_deleted)
            .await
    }

    async fn create_reply(&self, cmd: CreateReplyCommand) -> Result<ConfessionReply, DomainError> {
        // Verifier que la confession existe et n'est pas supprimee
        let conf = self
            .repo
            .get_confession(cmd.confession_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Confession introuvable".into()))?;
        if conf.deleted_at.is_some() {
            return Err(DomainError::ValidationError(
                "Confession supprimee, replies fermees".into(),
            ));
        }
        let cfg = self.config_or_default(&conf.guild_id).await?;
        cfg.validate_content(&cmd.content)
            .map_err(DomainError::ValidationError)?;
        if cfg.is_user_banned(&cmd.author_user_id) {
            return Err(DomainError::Forbidden(
                "Tu n'es plus autorise a poster".into(),
            ));
        }

        let public_number = self
            .repo
            .next_reply_public_number(cmd.confession_id)
            .await?;
        let reply = ConfessionReply {
            id: Uuid::new_v4(),
            confession_id: cmd.confession_id,
            public_number,
            author_user_id: cmd.author_user_id,
            content: cmd.content.trim().to_string(),
            is_anonymous: cmd.is_anonymous,
            message_id: None,
            deleted_at: None,
            deleted_by: None,
            edited_at: None,
            created_at: Utc::now(),
        };
        self.repo.create_reply(&reply).await?;
        Ok(reply)
    }

    async fn update_reply_message_id(
        &self,
        id: Uuid,
        message_id: String,
    ) -> Result<(), DomainError> {
        self.repo.update_reply_message_id(id, &message_id).await
    }

    async fn delete_reply(
        &self,
        id: Uuid,
        deleted_by: String,
    ) -> Result<ConfessionReply, DomainError> {
        let mut r = self
            .repo
            .get_reply(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Reply {} introuvable", id)))?;
        if r.deleted_at.is_some() {
            return Ok(r);
        }
        self.repo.soft_delete_reply(id, &deleted_by).await?;
        r.deleted_at = Some(Utc::now());
        r.deleted_by = Some(deleted_by);
        Ok(r)
    }

    async fn list_replies(&self, confession_id: Uuid) -> Result<Vec<ConfessionReply>, DomainError> {
        self.repo.list_replies(confession_id).await
    }

    async fn get_reply_parent_guild(&self, reply_id: Uuid) -> Result<String, DomainError> {
        let reply = self
            .repo
            .get_reply(reply_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Reply {} introuvable", reply_id)))?;
        let conf = self
            .repo
            .get_confession(reply.confession_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Confession parente introuvable".into()))?;
        Ok(conf.guild_id)
    }

    async fn create_report(
        &self,
        cmd: CreateReportCommand,
    ) -> Result<ConfessionReport, DomainError> {
        if cmd.confession_id.is_none() && cmd.reply_id.is_none() {
            return Err(DomainError::ValidationError(
                "Il faut une cible (confession ou reply)".into(),
            ));
        }
        if cmd.reason.trim().is_empty() {
            return Err(DomainError::ValidationError("Raison requise".into()));
        }
        let report = ConfessionReport {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            confession_id: cmd.confession_id,
            reply_id: cmd.reply_id,
            reporter_user_id: cmd.reporter_user_id,
            reason: cmd.reason.trim().to_string(),
            status: ReportStatus::Pending,
            resolved_by: None,
            resolved_at: None,
            created_at: Utc::now(),
        };
        self.repo.create_report(&report).await?;
        Ok(report)
    }

    async fn list_reports(
        &self,
        guild_id: &str,
        status: Option<ReportStatus>,
        limit: i64,
    ) -> Result<Vec<ConfessionReport>, DomainError> {
        self.repo.list_reports(guild_id, status, limit).await
    }

    async fn get_report_guild(&self, report_id: Uuid) -> Result<String, DomainError> {
        let report =
            self.repo.get_report(report_id).await?.ok_or_else(|| {
                DomainError::NotFound(format!("Report {} introuvable", report_id))
            })?;
        Ok(report.guild_id)
    }

    async fn resolve_report(
        &self,
        id: Uuid,
        status: ReportStatus,
        resolved_by: String,
    ) -> Result<(), DomainError> {
        self.repo.resolve_report(id, status, &resolved_by).await
    }

    async fn get_config(&self, guild_id: &str) -> Result<ConfessionConfig, DomainError> {
        self.config_or_default(guild_id).await
    }

    async fn save_config(&self, cfg: ConfessionConfig) -> Result<ConfessionConfig, DomainError> {
        if cfg.cooldown_secs < 0 || cfg.cooldown_secs > 3600 {
            return Err(DomainError::ValidationError(
                "cooldown_secs doit etre 0..3600".into(),
            ));
        }
        if cfg.max_per_day < 1 || cfg.max_per_day > 1000 {
            return Err(DomainError::ValidationError(
                "max_per_day doit etre 1..1000".into(),
            ));
        }
        if cfg.min_chars < 1 || cfg.min_chars > cfg.max_chars {
            return Err(DomainError::ValidationError(
                "min_chars doit etre >= 1 et <= max_chars".into(),
            ));
        }
        if cfg.max_chars < 1 || cfg.max_chars > 4000 {
            return Err(DomainError::ValidationError(
                "max_chars doit etre 1..4000".into(),
            ));
        }
        self.repo.upsert_config(&cfg).await?;
        Ok(cfg)
    }
}

#[cfg(test)]
#[path = "tests/manage_confessions_extended.rs"]
mod tests_extended;
