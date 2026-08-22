
use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use tracing::warn;

use crate::sentinel::domain::entities::audit::security_event::SecurityEvent;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::domain::services::audit::security_analyzer;
use crate::sentinel::ports::inbound::audit::manage_audit_logs::CreateAuditLogCommand;
use crate::sentinel::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase;
use crate::sentinel::ports::inbound::audit::manage_security::AnalyzeNewMemberCommand;
use crate::sentinel::ports::inbound::audit::manage_security::ManageSecurityUseCase;
use crate::sentinel::ports::inbound::audit::manage_security::ReportSecurityEventCommand;
use crate::sentinel::ports::inbound::audit::manage_security::SecurityDecision;
use crate::sentinel::ports::outbound::audit::security_event_repository::SecurityEventRepository;
use crate::sentinel::ports::outbound::audit::watched_user_repository::WatchedUserRepository;
use crate::sentinel::ports::outbound::moderation::moderation_repository::ModerationRepository;
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::sentinel::ports::outbound::system::cache::CachePort;
use crate::sentinel::ports::outbound::system::cache_helpers::cached_json;
const EVENTS_TTL: u64 = 60; // 1 minute

pub struct ManageSecurityService {
    repo: Arc<dyn SecurityEventRepository>,
    cache: Arc<dyn CachePort>,
    watched_repo: Arc<dyn WatchedUserRepository>,
    audit_logs_uc: Option<Arc<dyn ManageAuditLogsUseCase>>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
    moderation_repo: Arc<dyn ModerationRepository>,
}

impl ManageSecurityService {
    pub fn new(
        repo: Arc<dyn SecurityEventRepository>,
        cache: Arc<dyn CachePort>,
        watched_repo: Arc<dyn WatchedUserRepository>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
        moderation_repo: Arc<dyn ModerationRepository>,
    ) -> Self {
        Self {
            repo,
            cache,
            watched_repo,
            audit_logs_uc: None,
            bot_config_repo,
            moderation_repo,
        }
    }

    /// Phase 1 dual-write : copie chaque evenement de securite dans audit_logs
    /// avec event_type `security_<event_type>`.
    pub fn with_audit_logs_uc(mut self, audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>) -> Self {
        self.audit_logs_uc = Some(audit_logs_uc);
        self
    }
}

#[async_trait]
impl ManageSecurityUseCase for ManageSecurityService {
    async fn report_event(
        &self,
        cmd: ReportSecurityEventCommand,
    ) -> Result<SecurityEvent, DomainError> {
        let event = SecurityEvent {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            event_type: cmd.event_type,
            severity: cmd.severity,
            description: cmd.description,
            user_ids: cmd.user_ids,
            created_at: chrono::Utc::now(),
        };

        // Phase 4 : repo.save() est un no-op. La persistence est portee par
        // audit_logs_uc.create. Erreur dure si non injecte.
        self.repo.save(&event).await?;

        let uc = self.audit_logs_uc.as_ref().ok_or_else(|| {
            DomainError::Internal("audit_logs_uc non injecte dans ManageSecurityService".into())
        })?;
        let event_type_str = format!("security_{}", event.event_type);
        let details = serde_json::json!({
            "severity": event.severity,
            "description": event.description,
            "user_ids": event.user_ids,
            "event_id": event.id.to_string(),
        });
        let (target_id, target_name) = match event.user_ids.as_slice() {
            [single] => (Some(single.clone()), Some(single.clone())),
            _ => (None, None),
        };
        let cmd = CreateAuditLogCommand {
            guild_id: event.guild_id.clone(),
            event_type: event_type_str,
            actor_id: None,
            actor_name: None,
            target_id,
            target_name,
            channel_id: None,
            channel_name: None,
            details,
        };
        uc.create(cmd).await?;

        // Auto-surveillance : place chaque user concerne en manual watch.
        for uid in &event.user_ids {
            let reason = format!("Auto: {} ({})", event.event_type, event.severity);
            if let Err(e) = self
                .watched_repo
                .add_manual_watch(&event.guild_id, uid, uid, &reason, "security_event")
                .await
            {
                warn!(error = %e, guild_id = %event.guild_id, user_id = %uid, "Echec auto-surveillance");
            }
        }

        // Invalidate events cache
        if let Err(e) = self.cache.invalidate("security:all").await {
            warn!(error = %e, "Echec invalidation cache security:all");
        }
        if let Err(e) = self
            .cache
            .invalidate(&format!("security:{}", event.guild_id))
            .await
        {
            warn!(error = %e, guild_id = %event.guild_id, "Echec invalidation cache security guild");
        }

        Ok(event)
    }

    async fn analyze_new_member(
        &self,
        cmd: AnalyzeNewMemberCommand,
    ) -> Result<SecurityDecision, DomainError> {
        // Bots ignores.
        if cmd.is_bot {
            return Ok(SecurityDecision::default());
        }

        // Charger config guild.
        let configs = self
            .bot_config_repo
            .get_config(
                &cmd.guild_id,
                crate::sentinel::domain::entities::system::bot_names::SECURITY_BOT,
            )
            .await
            .unwrap_or_default();
        let cfg = |key: &str, default: u64| -> u64 {
            configs
                .iter()
                .find(|c| c.config_key == key)
                .and_then(|c| c.config_value.parse().ok())
                .unwrap_or(default)
        };
        let cfg_bool = |key: &str, default: bool| -> bool {
            configs
                .iter()
                .find(|c| c.config_key == key)
                .map(|c| c.config_value == "true" || c.config_value == "1")
                .unwrap_or(default)
        };

        let min_account_age = cfg("min_account_age_secs", 86400);
        let quarantine_enabled = cfg_bool("quarantine_enabled", false);
        let captcha_enabled = cfg_bool("captcha_enabled", false);
        let lockdown_enabled = cfg_bool("lockdown_enabled", false);
        let slowmode_secs = cfg("slowmode_seconds", 0) as u32;
        let alt_detection_enabled = cfg_bool("alt_detection_enabled", false);
        let raid_pattern_enabled = cfg_bool("raid_pattern_enabled", false);
        let raid_score_threshold = cfg("raid_pattern_score_threshold", 60) as u32;
        let name_distance = cfg("alt_name_distance", 2) as usize;
        let creation_spread = cfg("raid_creation_spread_secs", 3600) as i64;
        // Mode de reponse anti-raid (auto / suggest / hybrid) + seuil auto.
        let raid_mode = security_analyzer::RaidMode::from_config(
            configs
                .iter()
                .find(|c| c.config_key == "raid_mode")
                .map(|c| c.config_value.as_str())
                .unwrap_or(""),
        );
        let raid_auto_threshold = cfg("raid_auto_threshold", 85) as i32;

        let mut decision = SecurityDecision::default();

        // 1. Analyse raid pattern.
        if raid_pattern_enabled && cmd.recent_joins.len() >= 3 {
            let analysis =
                security_analyzer::analyze_joins(&cmd.recent_joins, name_distance, creation_spread);
            if analysis.score >= raid_score_threshold {
                decision.is_raid = true;
                decision.raid_score = analysis.score;
                decision.activate_lockdown = lockdown_enabled;
                decision.slowmode_secs = slowmode_secs;
                decision.quarantine = quarantine_enabled;
                decision.send_captcha = quarantine_enabled && captcha_enabled;
                // Politique auto-vs-suggest sur la reponse GUILD-WIDE.
                // Le flood de vitesse detecte cote bot est propage ici via
                // `cmd.is_velocity_raid` : un flood force l'application auto en
                // mode hybrid (raid massif).
                decision.suggest_only = matches!(
                    security_analyzer::raid_response_mode(
                        analysis.score as i32,
                        cmd.is_velocity_raid,
                        raid_mode,
                        raid_auto_threshold,
                    ),
                    security_analyzer::RaidResponseMode::Suggest
                );
                decision.event_type = "raid_detected".into();
                decision.event_description = format!(
                    "Raid pattern detecte (score {}). Noms similaires: {}, Avatars par defaut: {}, Creation clusteree: {}",
                    analysis.score, analysis.similar_names, analysis.high_default_avatar_ratio, analysis.clustered_creation
                );
            }
        }

        // 1b. Flood de vitesse (detecte cote bot) sans pattern raid API.
        // Un flood est traite comme un raid massif : reponse GUILD-WIDE
        // (lockdown / slowmode / bump verification), avec la meme politique
        // auto-vs-suggest que le pattern raid. On ne dedouble pas si le pattern
        // raid a deja arme la reponse ci-dessus.
        if cmd.is_velocity_raid && !decision.is_raid {
            decision.is_raid = true;
            decision.raid_score = decision.raid_score.max(raid_score_threshold);
            decision.activate_lockdown = lockdown_enabled;
            decision.slowmode_secs = slowmode_secs;
            decision.quarantine = quarantine_enabled;
            decision.send_captcha = quarantine_enabled && captcha_enabled;
            // Score bas (0) : seul le signal velocity decide l'auto en hybrid.
            decision.suggest_only = matches!(
                security_analyzer::raid_response_mode(0, true, raid_mode, raid_auto_threshold,),
                security_analyzer::RaidResponseMode::Suggest
            );
            decision.event_type = "raid_detected".into();
            decision.event_description =
                "Flood de vitesse detecte (trop de joins en peu de temps)".into();
        }

        // 2. Compte suspect (trop jeune).
        if !decision.is_raid {
            let suspicious = security_analyzer::is_account_suspicious(
                cmd.account_created_timestamp,
                min_account_age,
            );
            if suspicious {
                decision.is_suspicious_account = true;
                decision.quarantine = quarantine_enabled;
                decision.send_captcha = quarantine_enabled && captcha_enabled;
                if decision.event_type.is_empty() {
                    decision.event_type = "suspicious_account".into();
                    let age_hours =
                        (chrono::Utc::now().timestamp() - cmd.account_created_timestamp) / 3600;
                    decision.event_description = format!(
                        "Compte suspect : age {} heures (min requis : {} heures)",
                        age_hours,
                        min_account_age / 3600
                    );
                }
            }
        }

        // 3. Alt detection (noms/dates proches de bans recents).
        if !decision.is_raid && alt_detection_enabled {
            // Charger les bans recents depuis audit_logs (7 derniers jours).
            let recent_bans = self.load_recent_ban_usernames(&cmd.guild_id).await;
            if !recent_bans.is_empty() {
                let alt = security_analyzer::check_alt_account(
                    &cmd.username,
                    cmd.account_created_timestamp,
                    &recent_bans,
                    name_distance,
                    creation_spread,
                );
                if alt.is_suspicious() {
                    decision.is_alt_account = true;
                    decision.alt_similar_to = alt
                        .similar_to_banned
                        .or(alt.creation_near_banned)
                        .unwrap_or_default();
                    decision.quarantine = quarantine_enabled;
                    decision.send_captcha = quarantine_enabled && captcha_enabled;
                    if decision.event_type.is_empty() {
                        decision.event_type = "alt_account_suspected".into();
                        decision.event_description = format!(
                            "Alt suspecte de {} (similaire a banni: {})",
                            cmd.username, decision.alt_similar_to
                        );
                    }
                }
            }
        }

        // Auto-report si un event a ete detecte.
        if !decision.event_type.is_empty() {
            let _ = self
                .report_event(ReportSecurityEventCommand {
                    guild_id: cmd.guild_id.clone(),
                    event_type: decision.event_type.clone(),
                    severity: if decision.is_raid { "critical" } else { "high" }.into(),
                    description: decision.event_description.clone(),
                    user_ids: vec![cmd.user_id.clone().into()],
                })
                .await;
        }

        Ok(decision)
    }

    async fn list_events(&self, guild_id: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError> {
        let cache_key = match guild_id {
            Some(gid) => format!("security:{gid}"),
            None => "security:all".to_string(),
        };

        cached_json(&self.cache, &cache_key, EVENTS_TTL, || async {
            match guild_id {
                Some(gid) => self.repo.find_by_guild(gid).await,
                None => self.repo.find_all().await,
            }
        })
        .await
    }

    async fn purge_events(&self, guild_id: &str) -> Result<(u64, u64), DomainError> {
        self.repo.purge_guild(guild_id).await
    }
}

impl ManageSecurityService {
    /// Charge les usernames et dates de creation des bans recents (7j)
    /// depuis le repo moderation pour l'alt detection.
    async fn load_recent_ban_usernames(
        &self,
        guild_id: &str,
    ) -> Vec<security_analyzer::BannedUserInfo> {
        let bans = match self.moderation_repo.find_bans(Some(guild_id), 100, 0).await {
            Ok(actions) => actions,
            Err(_) => return vec![],
        };
        let seven_days_ago = chrono::Utc::now() - chrono::Duration::days(7);
        bans.into_iter()
            .filter(|a| a.created_at >= seven_days_ago)
            .map(|a| security_analyzer::BannedUserInfo {
                username: a.target_name,
                account_created_timestamp: 0,
            })
            .collect()
    }
}
