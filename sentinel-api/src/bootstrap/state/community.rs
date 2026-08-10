//! Etat du domaine community : vie du serveur, membres, roles, progression.
//!
//! C'est le plus large des domaines parce que c'est le plus large des sujets :
//! tout ce qui anime le serveur sans relever de la sanction (moderation) ni de
//! l'observation (audit).

use std::sync::Arc;

use axum::extract::FromRef;
use sentinel_core::ports::inbound::community::check_eligibility::CheckEligibilityUseCase;
use sentinel_core::ports::inbound::community::evaluate_age_declaration::EvaluateAgeDeclarationUseCase;
use sentinel_core::ports::inbound::community::manage_announcements::ManageAnnouncementsUseCase;
use sentinel_core::ports::inbound::community::manage_confessions::ManageConfessionsUseCase;
use sentinel_core::ports::inbound::community::manage_embeds::ManageEmbedsUseCase;
use sentinel_core::ports::inbound::community::manage_events::ManageEventsUseCase;
use sentinel_core::ports::inbound::community::manage_ideas::ManageIdeasUseCase;
use sentinel_core::ports::inbound::community::manage_levels::ManageLevelsUseCase;
use sentinel_core::ports::inbound::community::manage_lfg::ManageLfgUseCase;
use sentinel_core::ports::inbound::community::manage_members::ManageMembersUseCase;
use sentinel_core::ports::inbound::community::manage_monthly_ranking::ManageMonthlyRankingUseCase;
use sentinel_core::ports::inbound::community::manage_news::ManageNewsUseCase;
use sentinel_core::ports::inbound::community::manage_polls::ManagePollsUseCase;
use sentinel_core::ports::inbound::community::manage_sponsorships::ManageSponsorshipsUseCase;
use sentinel_core::ports::inbound::community::manage_spotlight::ManageSpotlightUseCase;
use sentinel_core::ports::inbound::community::manage_welcome_config::ManageWelcomeConfigUseCase;
use sentinel_core::ports::inbound::community::read_presence::ReadPresenceUseCase;
use sentinel_core::ports::outbound::community::age_ban_repository::AgeBanRepository;
use sentinel_core::ports::outbound::community::daily_activity_repository::DailyActivityRepository;
use sentinel_core::ports::outbound::community::discord_role_repository::DiscordRoleRepository;
use sentinel_core::ports::outbound::community::sponsorship_repository::SponsorshipRepository;
use sentinel_core::ports::outbound::community::temp_role_repository::TempRoleRepository;
use sentinel_core::ports::outbound::system::bot_config_repository::BotConfigRepository;

use crate::adapters::outbound::discord_api::DiscordApi;
use crate::adapters::outbound::ws::broadcaster::EventBroadcaster;
use crate::bootstrap::state::AppState;

/// Ports de la vie communautaire du serveur.
#[derive(Clone)]
pub struct CommunityState {
    // ── Vie du serveur ──
    pub events_uc: Arc<dyn ManageEventsUseCase>,
    pub lfg_uc: Arc<dyn ManageLfgUseCase>,
    /// Membres (DB-backed) : liste, profil, sync, lifecycle join/leave, reset.
    /// Surface HTTP `/api/members/*` + `/api/guilds/{id}/members` (via Discord).
    pub members_uc: Arc<dyn ManageMembersUseCase>,
    pub polls_uc: Arc<dyn ManagePollsUseCase>,
    pub spotlight_uc: Arc<dyn ManageSpotlightUseCase>,
    pub news_uc: Arc<dyn ManageNewsUseCase>,
    pub ideas_uc: Arc<dyn ManageIdeasUseCase>,
    pub confessions_uc: Arc<dyn ManageConfessionsUseCase>,
    pub announcements_uc: Arc<dyn ManageAnnouncementsUseCase>,
    pub embeds_uc: Arc<dyn ManageEmbedsUseCase>,
    /// Presence en direct, publiee par le bot dans Redis. Alimente une page
    /// PUBLIQUE : le bot ne publie que les salons visibles par `@everyone`.
    pub presence_uc: Arc<dyn ReadPresenceUseCase>,

    // ── Membres, roles, progression ──
pub levels_uc: Arc<dyn ManageLevelsUseCase>,
    pub monthly_ranking_uc: Arc<dyn ManageMonthlyRankingUseCase>,
    pub welcome_config_uc: Arc<dyn ManageWelcomeConfigUseCase>,
    pub eligibility_uc: Arc<dyn CheckEligibilityUseCase>,
    /// Verification d'age : decision server-side (seuil pass/ban + duree).
    pub age_check_uc: Arc<dyn EvaluateAgeDeclarationUseCase>,
    pub manage_sponsorships_uc: Arc<dyn ManageSponsorshipsUseCase>,

    // ── Repositories exposes directement ──
    pub daily_activity_repo: Arc<dyn DailyActivityRepository>,
    pub discord_role_repo: Arc<dyn DiscordRoleRepository>,
    pub age_ban_repo: Arc<dyn AgeBanRepository>,
    pub sponsorship_repo: Arc<dyn SponsorshipRepository>,
    pub temp_role_repo: Arc<dyn TempRoleRepository>,

    // ── Dependances transverses du domaine ──
    pub broadcaster: Arc<EventBroadcaster>,
    pub discord_api: Arc<dyn DiscordApi>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    /// Bus d'evenements Redis (`sentinel:events`) : annonces, embeds et idees
    /// sont postes sur Discord PAR LE BOT, pas par l'API. Cf. CLAUDE.md,
    /// section « deux chemins pour agir sur Discord depuis le web ».
    pub redis_client: redis::Client,
}

impl FromRef<AppState> for CommunityState {
    fn from_ref(state: &AppState) -> Self {
        state.community.clone()
    }
}


