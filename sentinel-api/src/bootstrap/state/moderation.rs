//! Etat du domaine moderation : regles, infractions, sanctions, reviews automod.

use std::sync::Arc;

use axum::extract::FromRef;
use sentinel_core::ports::inbound::moderation::assess_target_risk::AssessTargetRiskUseCase;
use sentinel_core::ports::inbound::moderation::cancel_action::CancelModerationActionUseCase;
use sentinel_core::ports::inbound::moderation::manage_automod_reviews::ManageAutomodReviewsUseCase;
use sentinel_core::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use sentinel_core::ports::inbound::moderation::manage_notes::ManageNotesUseCase;
use sentinel_core::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use sentinel_core::ports::inbound::moderation::manage_rules::ManageRulesUseCase;
use sentinel_core::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase;
use sentinel_core::ports::inbound::moderation::manage_sursis::ManageSursisUseCase;
use sentinel_core::ports::inbound::moderation::read_modstats::ReadModstatsUseCase;
use sentinel_core::ports::outbound::audit::modstats_repository::ModstatsRepository;
use sentinel_core::ports::outbound::moderation::adaptive_slowmode_repository::AdaptiveSlowmodeRepository;
use sentinel_core::ports::outbound::moderation::evidence_repository::EvidenceRepository;
use sentinel_core::ports::outbound::moderation::pending_action_repository::PendingActionRepository;
use sentinel_core::ports::outbound::moderation::review_repository::ReviewRepository;
use sentinel_core::ports::outbound::system::bot_config_repository::BotConfigRepository;

use crate::adapters::outbound::discord_api::DiscordApi;
use crate::adapters::outbound::ws::broadcaster::EventBroadcaster;
use crate::bootstrap::state::AppState;

/// Ports de la moderation manuelle et automatique.
///
/// Les trois dernieres dependances sont transverses mais bien reelles :
/// une sanction diffuse un evenement (`broadcaster`), applique un ban ou un
/// mute cote Discord (`discord_api`) et lit les seuils du serveur
/// (`bot_config_repo`). Les ecrire ici les rend visibles.
///
/// Ce qui n'y figure PAS est aussi une decision : `handlers/moderation/purge.rs`
/// purge les audit-logs et les logs systeme, donc il reste sur `AppState` et
/// suivra les domaines `audit` / `system`. L'y forcer aurait reconstitue un
/// god-object en miniature.
#[derive(Clone)]
pub struct ModerationState {
    pub rules_uc: Arc<dyn ManageRulesUseCase>,
    pub infractions_uc: Arc<dyn ManageInfractionsUseCase>,
    pub moderation_uc: Arc<dyn ManageModerationUseCase>,
    pub modstats_uc: Arc<dyn ReadModstatsUseCase>,
    /// Evaluation server-side du risque d'une cible (seuil + politique de
    /// confirmation). Le bot fournit les faits Discord, l'API decide.
    pub assess_target_risk_uc: Arc<dyn AssessTargetRiskUseCase>,
    pub automod_reviews_uc: Arc<dyn ManageAutomodReviewsUseCase>,
    pub automod_adaptive_slowmode_repo: Arc<dyn AdaptiveSlowmodeRepository>,
    pub sursis_uc: Arc<dyn ManageSursisUseCase>,
    /// Strikes (avertissements a paliers) : config des seuils + strikes actifs
    /// par membre. Surface HTTP `/api/strikes/*`.
    pub strikes_uc: Arc<dyn ManageStrikesUseCase>,
    /// Notes moderateurs (contexte interne sur un membre). Surface `/api/notes/*`.
    pub notes_uc: Arc<dyn ManageNotesUseCase>,
    /// Annulation d'une action (unwarn) : orchestre l'effet Discord inverse
    /// puis la suppression. Partage par le handler HTTP et le service gRPC.
    pub cancel_action_uc: Arc<dyn CancelModerationActionUseCase>,
    pub evidence_repo: Arc<dyn EvidenceRepository>,
    pub review_repo: Arc<dyn ReviewRepository>,
    pub pending_action_repo: Arc<dyn PendingActionRepository>,
    pub modstats_repo: Arc<dyn ModstatsRepository>,
    pub manage_reminders_uc: Arc<dyn sentinel_core::ports::inbound::moderation::manage_reminders::ManageRemindersUseCase>,

    // ── Dependances transverses du domaine ──
    pub broadcaster: Arc<EventBroadcaster>,
    pub discord_api: Arc<dyn DiscordApi>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl ModerationState {
    /// Delai de rappel avant expiration d'une sanction, lu dans la config du
    /// serveur (cle `reminder_advance_secs` du bot `moderation-bot`).
    ///
    /// Defaut 1 h, y compris en cas d'erreur de lecture : un rappel envoye au
    /// mauvais moment vaut mieux qu'une sanction qui expire sans prevenir.
    pub async fn bot_config_reminder_advance_secs(&self, guild_id: &str) -> u64 {
        match self
            .bot_config_repo
            .get_config(guild_id, "moderation-bot")
            .await
        {
            Ok(entries) => entries
                .iter()
                .find(|e| e.config_key == "reminder_advance_secs")
                .and_then(|e| e.config_value.parse().ok())
                .unwrap_or(3600),
            Err(_) => 3600,
        }
    }
}

impl FromRef<AppState> for ModerationState {
    fn from_ref(state: &AppState) -> Self {
        state.moderation.clone()
    }
}

