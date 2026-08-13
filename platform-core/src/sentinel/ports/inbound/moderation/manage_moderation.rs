use async_trait::async_trait;

use crate::sentinel::domain::entities::moderation::action::applied::ModerationAction;
use crate::sentinel::domain::entities::moderation::action::applied::UserModerationHistory;
use crate::sentinel::domain::entities::moderation::action::reversal::ActionReversalInfo;
use crate::sentinel::domain::entities::moderation::action::strikes::StrikeResult;
use crate::sentinel::domain::entities::system::discord_ids::ChannelId;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::errors::DomainError;

pub struct LogModerationCommand {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub gravity: Option<String>,
    pub duration: Option<u64>,
}

/// Resultat agrégé d'un log_action : action persistée + strike result optionnel.
/// Permet d'internaliser l'orchestration action+strike dans le service plutôt
/// que dans le handler HTTP (atomicité d'ordonnancement).
pub struct LoggedModerationAction {
    pub action: ModerationAction,
    pub strike: Option<StrikeResult>,
}

#[async_trait]
pub trait ManageModerationUseCase: Send + Sync {
    async fn log_action(
        &self,
        command: LogModerationCommand,
    ) -> Result<ModerationAction, DomainError>;
    /// Variante atomique (du point de vue architecture) : enregistre l'action
    /// et applique immediatement le strike associe dans la meme sequence.
    /// Si le strike echoue l'action reste sauvee (compensation non-destructive)
    /// mais on retourne quand meme un resultat exploitable cote handler.
    ///
    /// Default impl : appelle `log_action` sans strike (retrocompat pour les
    /// stubs de test qui n'ont pas besoin du strike).
    async fn log_action_with_strike(
        &self,
        command: LogModerationCommand,
    ) -> Result<LoggedModerationAction, DomainError> {
        let action = self.log_action(command).await?;
        Ok(LoggedModerationAction {
            action,
            strike: None,
        })
    }
    async fn get_history(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<UserModerationHistory, DomainError>;
    async fn list_bans(
        &self,
        guild_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ModerationAction>, DomainError>;
    /// Liste toutes les actions de moderation pour une guild (journal unifie).
    async fn list_actions(
        &self,
        guild_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<ModerationAction>, DomainError>;
    async fn delete_bans_for_user(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<(), DomainError>;
    /// Supprime une action de moderation par son ID (unwarn, annulation).
    async fn delete_action(&self, id: uuid::Uuid) -> Result<bool, DomainError>;

    /// Recupere le guild_id de l'action de moderation (RBAC gate).
    /// Default : None (pour les stubs de test).
    async fn action_guild_id(&self, _action_id: uuid::Uuid) -> Result<Option<String>, DomainError> {
        Ok(None)
    }

    /// Recupere les infos necessaires pour reverser une action (annulation +
    /// reversal Discord). Default : None (pour les stubs de test).
    async fn find_action_for_reversal(
        &self,
        _action_id: uuid::Uuid,
    ) -> Result<Option<ActionReversalInfo>, DomainError> {
        Ok(None)
    }

    /// Nombre d'actions posees par ce moderateur sur la fenetre effective
    /// (`mod_action_window_secs`). Default : 0 (pour les stubs de test).
    async fn count_recent_mod_actions(
        &self,
        _guild_id: &str,
        _moderator_id: &str,
        _window_secs: i64,
    ) -> Result<i64, DomainError> {
        Ok(0)
    }
}
