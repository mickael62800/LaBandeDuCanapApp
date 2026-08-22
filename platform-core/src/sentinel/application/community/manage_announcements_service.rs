use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::sentinel::domain::entities::community::announcement::{
    compute_next_run_at, render_template, AnnouncementRun, ButtonInteraction, ChannelPostResult,
    ContentType, InterpolationContext, RecurrenceType, RunStatus, ScheduledAnnouncement,
};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_announcements::{
    CreateAnnouncementCommand, ManageAnnouncementsUseCase, RenderedAnnouncement, RenderedEmbed,
    RetentionCleanupSummary, UpdateAnnouncementCommand,
};
use crate::sentinel::ports::outbound::community::announcement_repository::AnnouncementRepository;
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;

use crate::sentinel::domain::entities::system::bot_names::ANNOUNCEMENTS_BOT;
const DEFAULT_FETCH_LIMIT_PER_GUILD: i64 = 50;

pub struct ManageAnnouncementsService {
    repo: Arc<dyn AnnouncementRepository>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl ManageAnnouncementsService {
    pub fn new(
        repo: Arc<dyn AnnouncementRepository>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self {
            repo,
            bot_config_repo,
        }
    }

    /// Lit `fetch_limit` du guild dans bot_guild_config
    /// (bot_name='announcements'). Defaut : 50. Clamp [1, 500].
    async fn fetch_limit_for_guild(&self, guild_id: &str) -> i64 {
        let cap = self
            .bot_config_repo
            .get_config(guild_id, ANNOUNCEMENTS_BOT)
            .await
            .ok()
            .and_then(|cfgs| {
                cfgs.into_iter()
                    .find(|c| c.config_key == "fetch_limit")
                    .and_then(|c| c.config_value.parse::<i64>().ok())
            })
            .unwrap_or(DEFAULT_FETCH_LIMIT_PER_GUILD);
        cap.clamp(1, crate::sentinel::application::validation::PAGE_LIMIT_MAX)
    }

    /// Lit une cle de config du guild (bot_name='announcements').
    async fn read_config_value(&self, guild_id: &str, key: &str) -> Option<String> {
        self.bot_config_repo
            .get_config(guild_id, ANNOUNCEMENTS_BOT)
            .await
            .ok()?
            .into_iter()
            .find(|c| c.config_key == key)
            .map(|c| c.config_value)
    }

    /// Limite Discord du footer d'embed (miroir de la contrainte SQL).
    fn validate_footer_text(footer: Option<&str>) -> Result<(), DomainError> {
        if footer.is_some_and(|f| f.chars().count() > 2048) {
            return Err(DomainError::ValidationError(
                "embed_footer_text trop long (max 2048)".into(),
            ));
        }
        Ok(())
    }

    fn validate_create(&self, cmd: &CreateAnnouncementCommand) -> Result<(), DomainError> {
        crate::sentinel::application::validation::validate_non_empty(&cmd.name, "name")?;
        if cmd.recurrence_hour > 23 {
            return Err(DomainError::ValidationError("hour doit etre 0-23".into()));
        }
        if cmd.recurrence_minute > 59 {
            return Err(DomainError::ValidationError("minute doit etre 0-59".into()));
        }
        if cmd.channel_ids.is_empty() {
            return Err(DomainError::ValidationError(
                "au moins 1 channel requis".into(),
            ));
        }
        // Bornes de taille (anti-DoS : contenu enorme x N salons x recurrence).
        if cmd.channel_ids.len() > 25 {
            return Err(DomainError::ValidationError("max 25 salons".into()));
        }
        if cmd.name.chars().count() > 100 {
            return Err(DomainError::ValidationError(
                "name trop long (max 100)".into(),
            ));
        }
        if cmd.content_text.chars().count() > 4000 {
            return Err(DomainError::ValidationError(
                "content_text trop long (max 4000)".into(),
            ));
        }
        Self::validate_footer_text(cmd.embed_footer_text.as_deref())?;
        match cmd.recurrence_type {
            RecurrenceType::Once => {
                if cmd.scheduled_at.is_none() {
                    return Err(DomainError::ValidationError(
                        "scheduled_at requis pour recurrence=once".into(),
                    ));
                }
            }
            RecurrenceType::Weekly => {
                let dow = cmd.recurrence_day_of_week.ok_or_else(|| {
                    DomainError::ValidationError("day_of_week requis pour weekly".into())
                })?;
                if dow > 6 {
                    return Err(DomainError::ValidationError(
                        "day_of_week doit etre 0-6".into(),
                    ));
                }
            }
            RecurrenceType::Monthly => {
                let dom = cmd.recurrence_day_of_month.ok_or_else(|| {
                    DomainError::ValidationError("day_of_month requis pour monthly".into())
                })?;
                if !(1..=31).contains(&dom) {
                    return Err(DomainError::ValidationError(
                        "day_of_month doit etre 1-31".into(),
                    ));
                }
            }
            RecurrenceType::Yearly => {
                let dom = cmd.recurrence_day_of_month.ok_or_else(|| {
                    DomainError::ValidationError("day_of_month requis pour yearly".into())
                })?;
                if !(1..=31).contains(&dom) {
                    return Err(DomainError::ValidationError(
                        "day_of_month doit etre 1-31".into(),
                    ));
                }
                let month = cmd.recurrence_month.ok_or_else(|| {
                    DomainError::ValidationError("month requis pour yearly".into())
                })?;
                if !(1..=12).contains(&month) {
                    return Err(DomainError::ValidationError("month doit etre 1-12".into()));
                }
            }
            RecurrenceType::Daily => {}
        }
        if let (Some(start), Some(end)) = (Some(Utc::now()), cmd.end_date) {
            if end <= start {
                return Err(DomainError::ValidationError(
                    "end_date doit etre dans le futur".into(),
                ));
            }
        }
        if cmd.content_type == ContentType::Embed
            && cmd
                .embed_title
                .as_deref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true)
            && cmd.content_text.trim().is_empty()
        {
            return Err(DomainError::ValidationError(
                "embed requiert au moins un titre ou une description".into(),
            ));
        }
        Ok(())
    }

    fn build_mentions_prefix(a: &ScheduledAnnouncement) -> String {
        let mut parts: Vec<String> = Vec::new();
        if a.mention_everyone {
            parts.push("@everyone".to_string());
        }
        if a.mention_here {
            parts.push("@here".to_string());
        }
        for rid in &a.mention_role_ids {
            parts.push(format!("<@&{}>", rid));
        }
        parts.join(" ")
    }

    fn render(a: &ScheduledAnnouncement, run_id: Uuid) -> RenderedAnnouncement {
        let ctx = InterpolationContext {
            now: Utc::now(),
            guild_name: "", // pas indispensable pour le payload, le bot pourra completer
        };
        let content_text = render_template(&a.content_text, &ctx);

        let embed = if a.content_type == ContentType::Embed {
            Some(RenderedEmbed {
                title: a.embed_title.as_deref().map(|s| render_template(s, &ctx)),
                description: render_template(&a.content_text, &ctx),
                color: a.embed_color,
                image_url: a.embed_image_url.clone(),
                thumbnail_url: a.embed_thumbnail_url.clone(),
                footer_text: a
                    .embed_footer_text
                    .as_deref()
                    .map(|s| render_template(s, &ctx)),
            })
        } else {
            None
        };

        RenderedAnnouncement {
            announcement_id: a.id,
            run_id,
            guild_id: a.guild_id.clone(),
            channel_ids: a.channel_ids.clone(),
            content_text,
            embed,
            mentions_prefix: Self::build_mentions_prefix(a),
            buttons: a.buttons.clone(),
            auto_reactions: a.auto_reactions.clone(),
        }
    }
}

#[async_trait]
impl ManageAnnouncementsUseCase for ManageAnnouncementsService {
    async fn create(
        &self,
        cmd: CreateAnnouncementCommand,
    ) -> Result<ScheduledAnnouncement, DomainError> {
        self.validate_create(&cmd)?;
        let now = Utc::now();
        let next = compute_next_run_at(
            cmd.recurrence_type,
            cmd.recurrence_hour,
            cmd.recurrence_minute,
            cmd.recurrence_day_of_week,
            cmd.recurrence_day_of_month,
            cmd.recurrence_month,
            cmd.scheduled_at,
            cmd.end_date,
            now,
        )
        .ok_or_else(|| {
            DomainError::ValidationError(
                "Impossible de planifier (date passe / end_date invalide)".into(),
            )
        })?;

        let ann = ScheduledAnnouncement {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            name: cmd.name,
            enabled: true,
            recurrence_type: cmd.recurrence_type,
            recurrence_hour: cmd.recurrence_hour,
            recurrence_minute: cmd.recurrence_minute,
            recurrence_day_of_week: cmd.recurrence_day_of_week,
            recurrence_day_of_month: cmd.recurrence_day_of_month,
            recurrence_month: cmd.recurrence_month,
            scheduled_at: cmd.scheduled_at,
            start_date: now,
            end_date: cmd.end_date,
            content_type: cmd.content_type,
            content_text: cmd.content_text,
            embed_title: cmd.embed_title,
            embed_color: cmd.embed_color,
            embed_image_url: cmd.embed_image_url,
            embed_thumbnail_url: cmd.embed_thumbnail_url,
            embed_footer_text: cmd.embed_footer_text,
            mention_everyone: cmd.mention_everyone,
            mention_here: cmd.mention_here,
            mention_role_ids: cmd.mention_role_ids,
            channel_ids: cmd.channel_ids,
            buttons: cmd.buttons,
            auto_reactions: cmd.auto_reactions,
            created_by: cmd.created_by,
            created_at: now,
            updated_at: now,
            last_run_at: None,
            next_run_at: next,
        };
        self.repo.create(&ann).await?;
        Ok(ann)
    }

    async fn update(
        &self,
        cmd: UpdateAnnouncementCommand,
    ) -> Result<ScheduledAnnouncement, DomainError> {
        Self::validate_footer_text(cmd.embed_footer_text.as_deref())?;

        let mut ann = self
            .repo
            .get_by_id(cmd.id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Annonce {} introuvable", cmd.id)))?;

        let next = compute_next_run_at(
            cmd.recurrence_type,
            cmd.recurrence_hour,
            cmd.recurrence_minute,
            cmd.recurrence_day_of_week,
            cmd.recurrence_day_of_month,
            cmd.recurrence_month,
            cmd.scheduled_at,
            cmd.end_date,
            Utc::now(),
        )
        .ok_or_else(|| {
            DomainError::ValidationError(
                "Impossible de planifier (date passe / end_date invalide)".into(),
            )
        })?;

        ann.name = cmd.name;
        ann.recurrence_type = cmd.recurrence_type;
        ann.recurrence_hour = cmd.recurrence_hour;
        ann.recurrence_minute = cmd.recurrence_minute;
        ann.recurrence_day_of_week = cmd.recurrence_day_of_week;
        ann.recurrence_day_of_month = cmd.recurrence_day_of_month;
        ann.recurrence_month = cmd.recurrence_month;
        ann.scheduled_at = cmd.scheduled_at;
        ann.end_date = cmd.end_date;
        ann.content_type = cmd.content_type;
        ann.content_text = cmd.content_text;
        ann.embed_title = cmd.embed_title;
        ann.embed_color = cmd.embed_color;
        ann.embed_image_url = cmd.embed_image_url;
        ann.embed_thumbnail_url = cmd.embed_thumbnail_url;
        ann.embed_footer_text = cmd.embed_footer_text;
        ann.mention_everyone = cmd.mention_everyone;
        ann.mention_here = cmd.mention_here;
        ann.mention_role_ids = cmd.mention_role_ids;
        ann.channel_ids = cmd.channel_ids;
        ann.buttons = cmd.buttons;
        ann.auto_reactions = cmd.auto_reactions;
        ann.updated_at = Utc::now();
        ann.next_run_at = next;

        self.repo.update(&ann).await?;
        Ok(ann)
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        self.repo.delete(id).await
    }

    async fn get(&self, id: Uuid) -> Result<ScheduledAnnouncement, DomainError> {
        self.repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Annonce {} introuvable", id)))
    }

    async fn list_by_guild(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ScheduledAnnouncement>, DomainError> {
        self.repo.list_by_guild(guild_id).await
    }

    async fn toggle(&self, id: Uuid, enabled: bool) -> Result<bool, DomainError> {
        self.repo.set_enabled(id, enabled).await
    }

    async fn fetch_due_and_prepare(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<RenderedAnnouncement>, DomainError> {
        // 1) Fetch global avec une borne haute (limit du caller : ceiling
        //    pour ne pas exploser la memoire si toutes les guilds ont
        //    plein d'annonces dues le meme tick).
        let due = self.repo.list_due(now, limit).await?;

        // 2) Group par guild + cap par-guild via `fetch_limit` config.
        //    Skip les annonces au-dela du cap (elles seront re-piquees au
        //    prochain tick — leur next_run_at n'est pas avance).
        use std::collections::HashMap;
        let mut counts: HashMap<String, i64> = HashMap::new();
        let mut guild_caps: HashMap<String, i64> = HashMap::new();
        let mut kept: Vec<&ScheduledAnnouncement> = Vec::with_capacity(due.len());
        for a in &due {
            let cap = match guild_caps.get(&a.guild_id) {
                Some(c) => *c,
                None => {
                    let c = self.fetch_limit_for_guild(&a.guild_id).await;
                    guild_caps.insert(a.guild_id.clone(), c);
                    c
                }
            };
            let count = counts.entry(a.guild_id.clone()).or_insert(0);
            if *count >= cap {
                continue;
            }
            *count += 1;
            kept.push(a);
        }

        let mut rendered = Vec::with_capacity(kept.len());
        for &a in &kept {
            let run = AnnouncementRun {
                id: Uuid::new_v4(),
                announcement_id: a.id,
                guild_id: a.guild_id.clone(),
                ran_at: now,
                channels_posted: Vec::new(),
                status: RunStatus::Pending,
                error: None,
            };
            self.repo.insert_run(&run).await?;
            rendered.push(Self::render(a, run.id));
            // Calcule la prochaine occurrence et l'inscrit deja en BDD pour
            // que le worker ne re-pick pas cette annonce a un tick proche
            // (skip rule : on ne rattrape pas).
            let next = compute_next_run_at(
                a.recurrence_type,
                a.recurrence_hour,
                a.recurrence_minute,
                a.recurrence_day_of_week,
                a.recurrence_day_of_month,
                a.recurrence_month,
                a.scheduled_at,
                a.end_date,
                now,
            );
            self.repo.mark_run(a.id, now, next).await?;
        }
        Ok(rendered)
    }

    async fn record_run_result(
        &self,
        run_id: Uuid,
        channels_posted: Vec<ChannelPostResult>,
    ) -> Result<(), DomainError> {
        let any_fail = channels_posted.iter().any(|c| !c.success);
        let all_fail = !channels_posted.is_empty() && channels_posted.iter().all(|c| !c.success);
        let status = if all_fail {
            RunStatus::Error
        } else if any_fail {
            RunStatus::Partial
        } else {
            RunStatus::Success
        };
        let error_summary = if any_fail {
            channels_posted
                .iter()
                .filter_map(|c| c.error.clone())
                .collect::<Vec<_>>()
                .join(" | ")
        } else {
            String::new()
        };
        let error_opt = if error_summary.is_empty() {
            None
        } else {
            Some(error_summary.as_str())
        };
        self.repo
            .update_run_result(run_id, status, &channels_posted, error_opt)
            .await
    }

    async fn preview(&self, id: Uuid) -> Result<RenderedAnnouncement, DomainError> {
        let ann = self.get(id).await?;
        Ok(Self::render(&ann, Uuid::nil()))
    }

    async fn list_runs(
        &self,
        announcement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AnnouncementRun>, DomainError> {
        self.repo.list_runs(announcement_id, limit).await
    }

    async fn record_button_interaction(
        &self,
        announcement_id: Uuid,
        run_id: Option<Uuid>,
        user_id: String,
        user_name: Option<String>,
        button_custom_id: String,
        button_label: Option<String>,
    ) -> Result<(), DomainError> {
        let interaction = ButtonInteraction {
            id: Uuid::new_v4(),
            announcement_id,
            run_id,
            user_id,
            user_name,
            button_custom_id,
            button_label,
            clicked_at: Utc::now(),
        };
        self.repo.record_button_interaction(&interaction).await
    }

    async fn list_button_interactions(
        &self,
        announcement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ButtonInteraction>, DomainError> {
        self.repo
            .list_button_interactions(announcement_id, limit)
            .await
    }

    async fn retention_cleanup_all(&self) -> Result<RetentionCleanupSummary, DomainError> {
        let guild_ids = self.repo.list_guild_ids().await?;

        let mut guilds_processed = 0u64;
        let mut guilds_skipped = 0u64;
        let mut rows_deleted: i64 = 0;

        for guild_id in &guild_ids {
            // Module actif ?
            let enabled = self.read_config_value(guild_id, "enabled").await;
            let active = !matches!(enabled.as_deref(), Some("false") | Some("0"));
            if !active {
                guilds_skipped += 1;
                continue;
            }
            // history_retention_days (defaut 90). <= 0 = illimite -> skip.
            let retention = self
                .read_config_value(guild_id, "history_retention_days")
                .await
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(90);
            if retention <= 0 {
                guilds_skipped += 1;
                continue;
            }
            match self
                .repo
                .delete_runs_older_than(guild_id, retention as i32)
                .await
            {
                Ok(deleted) => {
                    rows_deleted += deleted as i64;
                    guilds_processed += 1;
                }
                Err(e) => {
                    tracing::warn!(error = %e, guild = %guild_id, "retention announcement_runs echec");
                }
            }
        }

        Ok(RetentionCleanupSummary {
            guilds_processed,
            guilds_skipped,
            rows_deleted,
        })
    }
}


