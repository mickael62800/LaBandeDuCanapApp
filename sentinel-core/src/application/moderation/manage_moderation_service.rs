use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::moderation::action::applied::ModerationAction;
use crate::domain::entities::moderation::action::applied::UserModerationHistory;
use crate::domain::errors::DomainError;
use crate::ports::inbound::audit::manage_audit_logs::CreateAuditLogCommand;
use crate::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase;
use crate::ports::inbound::moderation::manage_moderation::LogModerationCommand;
use crate::ports::inbound::moderation::manage_moderation::LoggedModerationAction;
use crate::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use crate::ports::inbound::moderation::manage_strikes::AddStrikeCommand;
use crate::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase;
use tracing::warn;

use crate::ports::outbound::moderation::moderation_repository::ModerationRepository;
use crate::ports::outbound::moderation::strike_repository::StrikeRepository;
use crate::ports::outbound::system::cache::CachePort;
use crate::ports::outbound::system::cache_helpers::cached_json;
const HISTORY_TTL: u64 = 180; // 3 minutes

pub struct ManageModerationService {
    repo: Arc<dyn ModerationRepository>,
    strike_repo: Arc<dyn StrikeRepository>,
    cache: Arc<dyn CachePort>,
    strikes_uc: Option<Arc<dyn ManageStrikesUseCase>>,
    audit_logs_uc: Option<Arc<dyn ManageAuditLogsUseCase>>,
}

impl ManageModerationService {
    pub fn new(
        repo: Arc<dyn ModerationRepository>,
        strike_repo: Arc<dyn StrikeRepository>,
        cache: Arc<dyn CachePort>,
    ) -> Self {
        Self {
            repo,
            strike_repo,
            cache,
            strikes_uc: None,
            audit_logs_uc: None,
        }
    }

    /// Injecte le use case strikes (optionnel — active `log_action_with_strike`).
    /// Builder-style pour eviter une dependance circulaire a la construction
    /// (strikes_uc n'existe pas encore quand ManageModerationService::new est
    /// appele dans main.rs).
    pub fn with_strikes_uc(mut self, strikes_uc: Arc<dyn ManageStrikesUseCase>) -> Self {
        self.strikes_uc = Some(strikes_uc);
        self
    }

    /// Injecte le use case audit logs (Phase 1 dual-write : chaque action de
    /// moderation est aussi loggee dans audit_logs avec event_type `mod_<action>`).
    pub fn with_audit_logs_uc(mut self, audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>) -> Self {
        self.audit_logs_uc = Some(audit_logs_uc);
        self
    }
}

#[async_trait]
impl ManageModerationUseCase for ManageModerationService {
    async fn log_action(&self, cmd: LogModerationCommand) -> Result<ModerationAction, DomainError> {
        // Truncate raison a 500 chars AVANT persist (pas seulement dans l'embed).
        // Evite que la DB contienne plus de texte que ce que les UIs peuvent afficher.
        let truncated_reason: String = cmd.reason.chars().take(500).collect();

        let action = ModerationAction {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            channel_id: cmd.channel_id,
            moderator_id: cmd.moderator_id,
            moderator_name: cmd.moderator_name,
            target_id: cmd.target_id.clone(),
            target_name: cmd.target_name,
            target_display_name: None,
            action_type: cmd.action_type,
            reason: truncated_reason,
            gravity: cmd.gravity.as_deref().and_then(crate::domain::enums::moderation::moderation_gravity::ModerationGravity::from_str_lossy),
            duration: cmd.duration,
            created_at: chrono::Utc::now(),
        };


        let uc = self.audit_logs_uc.as_ref().ok_or_else(|| {
            DomainError::Internal("audit_logs_uc non injecte dans ManageModerationService".into())
        })?;
        let event_type = format!("mod_{}", action.action_type);
        let details = serde_json::json!({
            "reason": action.reason,
            "gravity": action.gravity.as_ref().map(|g| g.as_str()),
            "duration_secs": action.duration,
            "action_id": action.id.to_string(),
        });
        let audit_cmd = CreateAuditLogCommand {
            guild_id: action.guild_id.clone(),
            event_type,
            actor_id: Some(action.moderator_id.clone()),
            actor_name: Some(action.moderator_name.clone()),
            target_id: Some(action.target_id.clone()),
            target_name: Some(action.target_name.clone()),
            channel_id: Some(action.channel_id.clone().into()),
            channel_name: None,
            details,
        };
        uc.create(audit_cmd).await?;

        // Invalidate history cache for this user
        let cache_key = format!("modhistory:{}:{}", action.guild_id, action.target_id);
        if let Err(e) = self.cache.invalidate(&cache_key).await {
            warn!(error = %e, cache_key = %cache_key, "Echec invalidation cache mod history");
        }

        Ok(action)
    }

    async fn log_action_with_strike(
        &self,
        cmd: LogModerationCommand,
    ) -> Result<LoggedModerationAction, DomainError> {
        // Capture les champs necessaires pour la commande strike AVANT le move.
        let guild_id = cmd.guild_id.clone();
        let target_id = cmd.target_id.clone();
        let reason = cmd.reason.clone();
        let action_type = cmd.action_type.clone();

        let action = self.log_action(cmd).await?;

        // La "prevention" est tracee dans l'historique mais NE compte PAS dans
        // l'escalade : on n'ajoute pas de strike (cran sous le warn).
        if action_type == "prevention" {
            return Ok(LoggedModerationAction {
                action,
                strike: None,
            });
        }

        // Si le strikes_uc n'a pas ete injecte, on retourne sans strike
        // (compat descendante : certains tests n'en ont pas besoin).
        let strike = match &self.strikes_uc {
            Some(uc) => {
                match uc
                    .add_strike(AddStrikeCommand {
                        guild_id,
                        user_id: target_id.into(),
                        reason,
                        source: "moderator".into(),
                        infraction_id: Some(action.id),
                    })
                    .await
                {
                    Ok(r) => Some(r),
                    Err(e) => {
                        // Compensation non destructive : l'action reste, on log
                        // l'incoherence pour alerting.
                        tracing::error!(
                            error = %e,
                            action_id = %action.id,
                            guild_id = %action.guild_id,
                            target_id = %action.target_id,
                            "INCOHERENCE : action sauvee mais strike echoue"
                        );
                        None
                    }
                }
            }
            None => None,
        };

        Ok(LoggedModerationAction { action, strike })
    }

    async fn get_history(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<UserModerationHistory, DomainError> {
        let cache_key = format!("modhistory:{guild_id}:{target_id}");
        cached_json(&self.cache, &cache_key, HISTORY_TTL, || async {
            let actions = self.repo.find_by_target(guild_id, target_id, 500).await?;
            let target_name = actions
                .first()
                .map(|a| a.target_name.clone())
                .unwrap_or_default();

            let total_warns = actions.iter().filter(|a| a.action_type == "warn").count() as u32;
            let total_mutes = actions
                .iter()
                .filter(|a| a.action_type.starts_with("mute"))
                .count() as u32;
            let total_bans = actions
                .iter()
                .filter(|a| a.action_type.starts_with("ban"))
                .count() as u32;

            Ok(UserModerationHistory {
                target_id: target_id.to_string(),
                target_name,
                total_warns,
                total_mutes,
                total_bans,
                actions,
            })
        })
        .await
    }

    async fn list_bans(
        &self,
        guild_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        self.repo.find_bans(guild_id, limit, offset).await
    }

    async fn list_actions(
        &self,
        guild_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        self.repo.find_all_for_guild(guild_id, limit).await
    }

    async fn delete_bans_for_user(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<(), DomainError> {
        self.repo.delete_bans_for_user(guild_id, target_id).await?;
        let cache_key = format!("modhistory:{guild_id}:{target_id}");
        if let Err(e) = self.cache.invalidate(&cache_key).await {
            warn!(error = %e, cache_key = %cache_key, "Echec invalidation cache mod history apres delete_bans_for_user");
        }
        Ok(())
    }

    async fn delete_action(&self, id: uuid::Uuid) -> Result<bool, DomainError> {
        // Lire l'action avant suppression pour pouvoir invalider le cache cible
        // et supprimer le strike associe (lien via infraction_id = action.id).
        let action = match self.repo.find_by_id(id).await? {
            Some(a) => a,
            None => return Ok(false),
        };

        let deleted = self.repo.delete_action(id).await?;
        if !deleted {
            return Ok(false);
        }

        // Supprimer le strike lie (sinon l'escalation continue de compter
        // l'infraction qu'on vient de retirer).
        match self.strike_repo.delete_strike_by_infraction_id(id).await {
            Ok(count) if count > 0 => {
                tracing::info!(action_id = %id, strikes_removed = count, "Strikes lies a l'action supprimes");
            }
            Ok(_) => {}
            Err(e) => {
                warn!(error = %e, action_id = %id, "Echec suppression strike lie a l'action");
            }
        }

        // Invalidation ciblee du cache modhistory pour ce user uniquement.
        let cache_key = format!("modhistory:{}:{}", action.guild_id, action.target_id);
        if let Err(e) = self.cache.invalidate(&cache_key).await {
            warn!(error = %e, cache_key = %cache_key, "Echec invalidation cache mod history apres delete_action");
        }

        Ok(true)
    }

    async fn action_guild_id(&self, action_id: uuid::Uuid) -> Result<Option<String>, DomainError> {
        self.repo.action_guild_id(action_id).await
    }

    async fn find_action_for_reversal(
        &self,
        action_id: uuid::Uuid,
    ) -> Result<
        Option<crate::domain::entities::moderation::action::reversal::ActionReversalInfo>,
        DomainError,
    > {
        self.repo.find_action_for_reversal(action_id).await
    }

    async fn count_recent_mod_actions(
        &self,
        guild_id: &str,
        moderator_id: &str,
        window_secs: i64,
    ) -> Result<i64, DomainError> {
        self.repo
            .count_recent_mod_actions(guild_id, moderator_id, window_secs)
            .await
    }
}

#[cfg(test)]
#[path = "tests/manage_moderation.rs"]
mod tests;
