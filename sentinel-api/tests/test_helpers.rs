//! Test helpers : construit un AppState complet avec des stubs pour tous les traits.
//! Seul le use case sous test est fonctionnel, les autres panic si appeles.
//!
//! # Pourquoi un `allow(dead_code)` global ici, et nulle part ailleurs
//!
//! Ce fichier est inclus par `#[path]` dans ~40 binaires de test, chacun
//! compile comme une crate independante. Un helper consomme par UN seul
//! binaire est donc « jamais utilise » dans les 39 autres, ce qui produit
//! ~800 avertissements pour zero ligne de code reellement morte.
//!
//! Ce n'est pas un contournement de complaisance : c'est la seule reponse a
//! une limite du modele de compilation des tests d'integration Rust. Il
//! remplace les 22 attributs cibles qui parsemaient le fichier — un allow
//! motive vaut mieux que vingt-deux muets.
//!
//! Nulle part ailleurs dans le workspace il ne reste de `allow(dead_code)` :
//! tout ce qu'ils masquaient a ete supprime ou justifie par `#[cfg(test)]`.
#![allow(dead_code)]

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use sentinel_api::adapters::outbound::discord_api::DiscordApi;
use sentinel_api::adapters::outbound::discord_api::DiscordApiService;
use sentinel_api::adapters::outbound::discord_api::DiscordChannel;
use sentinel_api::adapters::outbound::discord_api::DiscordMember;
use sentinel_api::adapters::outbound::discord_api::DiscordUser;
use sentinel_api::adapters::outbound::discord_api::UserGuild;
use sentinel_api::adapters::outbound::job_client::JobClient;
use sentinel_api::adapters::outbound::ws::broadcaster::EventBroadcaster;
use sentinel_api::bootstrap::state::AppState;
use sentinel_core::domain::entities::ai::image_analysis::*;
use sentinel_core::domain::entities::ai::message_analysis::*;
use sentinel_core::domain::entities::audit::audit_log::*;
use sentinel_core::domain::entities::audit::dashboard_stats::*;
use sentinel_core::domain::entities::audit::security_event::*;
use sentinel_core::domain::entities::audit::user_activity::*;
use sentinel_core::domain::entities::audit::user_stats::*;
use sentinel_core::domain::entities::audit::watched_user::*;
use sentinel_core::domain::entities::community::daily_activity::*;
use sentinel_core::domain::entities::community::guild_member::*;
use sentinel_core::domain::entities::community::level::*;
use sentinel_core::domain::entities::community::role_panel::*;
use sentinel_core::domain::entities::community::voice_channel::*;
use sentinel_core::domain::entities::moderation::action::applied::*;
use sentinel_core::domain::entities::moderation::action::sanction_reminder::*;
use sentinel_core::domain::entities::moderation::action::strikes::*;
use sentinel_core::domain::entities::moderation::infraction::*;
use sentinel_core::domain::entities::moderation::user_note::*;
use sentinel_core::domain::entities::system::analytics::*;
use sentinel_core::domain::entities::system::bot_config::*;
use sentinel_core::domain::entities::system::discord_role::*;
use sentinel_core::domain::entities::system::guild::*;
use ops_core::domain::entities::log_entry::*;
use sentinel_core::domain::entities::system::rule::*;
use sentinel_core::domain::entities::system::ticket::*;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::inbound::ai::analyze_image::*;
use sentinel_core::ports::inbound::ai::analyze_message::*;
use sentinel_core::ports::inbound::audit::manage_audit_logs::*;
use sentinel_core::ports::inbound::audit::manage_security::*;
use sentinel_core::ports::inbound::audit::manage_stats::*;
use sentinel_core::ports::inbound::audit::manage_watched_users::*;
use sentinel_core::ports::inbound::audit::*;
use sentinel_core::ports::inbound::community::manage_levels::*;
use sentinel_core::ports::inbound::community::manage_members::*;
use sentinel_core::ports::inbound::community::manage_role_panels::*;
use sentinel_core::ports::inbound::community::manage_voice_channels::*;
use sentinel_core::ports::inbound::community::*;
use sentinel_core::ports::inbound::moderation::manage_infractions::*;
use sentinel_core::ports::inbound::moderation::manage_moderation::*;
use sentinel_core::ports::inbound::moderation::manage_notes::*;
use sentinel_core::ports::inbound::moderation::manage_reminders::*;
use sentinel_core::ports::inbound::moderation::manage_rules::*;
use sentinel_core::ports::inbound::moderation::manage_strikes::*;
use sentinel_core::ports::inbound::moderation::*;
use sentinel_core::ports::inbound::system::manage_tickets::*;
use sentinel_core::ports::outbound::audit::analytics_repository::*;
use sentinel_core::ports::outbound::audit::modstats_repository::*;
use sentinel_core::ports::outbound::audit::user_activity_repository::*;
use sentinel_core::ports::outbound::community::daily_activity_repository::*;
use sentinel_core::ports::outbound::community::discord_role_repository::*;
use sentinel_core::ports::outbound::community::sponsorship_repository::*;
use sentinel_core::ports::outbound::community::temp_role_repository::*;
use sentinel_core::ports::outbound::community::welcome_config_repository::*;
use sentinel_core::ports::outbound::moderation::evidence_repository::*;
use sentinel_core::ports::outbound::moderation::pending_action_repository::*;
use sentinel_core::ports::outbound::moderation::review_repository::*;
use sentinel_core::ports::outbound::system::bot_config_repository::*;
use sentinel_core::ports::outbound::system::guild_repository::*;
use ops_core::ports::outbound::log_repository::*;

// ══════════════════════════════════════════════════════════
// Stub Use Cases (inbound)
// ══════════════════════════════════════════════════════════

pub struct StubAnalyzeMessage;
#[async_trait]
impl AnalyzeMessageUseCase for StubAnalyzeMessage {
    async fn analyze(&self, _: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError> {
        unimplemented!()
    }
    async fn evaluate_flood(&self, _: &str, _: i32) -> Result<FloodDecision, DomainError> {
        unimplemented!()
    }
    async fn evaluate_attachments(
        &self,
        _: &str,
        _: Vec<String>,
    ) -> Result<sentinel_core::ports::inbound::ai::analyze_message::AttachmentDecision, DomainError>
    {
        unimplemented!()
    }
    async fn evaluate_caps(
        &self,
        _: &str,
    ) -> Result<sentinel_core::ports::inbound::ai::analyze_message::CapsDecision, DomainError> {
        unimplemented!()
    }
}

pub struct StubAnalyzeImage;
#[async_trait]
impl AnalyzeImageUseCase for StubAnalyzeImage {
    async fn analyze_image(&self, _: AnalyzeImageCommand) -> Result<ImageAnalysis, DomainError> {
        unimplemented!()
    }
}

pub struct StubRules;
#[async_trait]
impl ManageRulesUseCase for StubRules {
    async fn get_rules(&self, _: &str) -> Result<Vec<Rule>, DomainError> {
        unimplemented!()
    }
    async fn get_all_rules(&self) -> Result<Vec<Rule>, DomainError> {
        unimplemented!()
    }
    async fn toggle_rule(&self, _: Uuid, _: bool) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn create_or_update_rule(&self, _: CreateRuleCommand) -> Result<Rule, DomainError> {
        unimplemented!()
    }
    async fn delete_rule(&self, _: &str, _: Uuid) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn seed_default_rules(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubInfractions;
#[async_trait]
impl ManageInfractionsUseCase for StubInfractions {
    async fn count_user_infractions(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        sentinel_core::ports::inbound::moderation::manage_infractions::UserInfractionCounts,
        DomainError,
    > {
        unimplemented!("count_user_infractions non exerce par ces tests")
    }
    async fn list_infractions(
        &self,
        _: &str,
        _: InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError> {
        unimplemented!()
    }
    async fn list_all_infractions(&self, _: i64, _: i64) -> Result<Vec<Infraction>, DomainError> {
        unimplemented!()
    }
    async fn count_today(&self) -> Result<u64, DomainError> {
        unimplemented!()
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<Infraction>, DomainError> {
        unimplemented!()
    }
    async fn delete_infraction(&self, _: &str) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, DomainError> {
        unimplemented!()
    }
}

pub struct StubTickets;
#[async_trait]
impl ManageTicketsUseCase for StubTickets {
    async fn list_tickets(
        &self,
        _: Option<String>,
        _: Option<String>,
        _: Option<String>,
        _: Option<String>,
        _: i64,
        _: i64,
    ) -> Result<Vec<Ticket>, DomainError> {
        unimplemented!()
    }
    async fn get_ticket_detail(&self, _: &str) -> Result<TicketDetail, DomainError> {
        unimplemented!()
    }
    async fn create_ticket(&self, _: CreateTicketCommand) -> Result<Ticket, DomainError> {
        unimplemented!()
    }
    async fn reply_ticket(&self, _: ReplyTicketCommand) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn close_ticket(&self, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn assign_ticket(&self, _: AssignTicketCommand) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn update_status(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn update_ticket_channel(
        &self,
        _: UpdateTicketChannelCommand,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn update_priority(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn update_sla(
        &self,
        _: Uuid,
        _: Option<&str>,
        _: Option<&str>,
        _: Option<i32>,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn moderated_guilds(
        &self,
        _: &str,
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        Ok(std::collections::HashSet::new())
    }
    async fn bulk_delete_tickets(
        &self,
        _: Option<&str>,
        _: Option<chrono::DateTime<chrono::Utc>>,
        _: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<u64, DomainError> {
        Ok(0)
    }
}

pub struct StubSecurity;
#[async_trait]
impl ManageSecurityUseCase for StubSecurity {
    async fn report_event(
        &self,
        _: ReportSecurityEventCommand,
    ) -> Result<SecurityEvent, DomainError> {
        unimplemented!()
    }
    async fn purge_events(&self, _: &str) -> Result<(u64, u64), DomainError> {
        Ok((0, 0))
    }
    async fn list_events(&self, _: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError> {
        unimplemented!()
    }
    async fn analyze_new_member(
        &self,
        _: AnalyzeNewMemberCommand,
    ) -> Result<SecurityDecision, DomainError> {
        unimplemented!()
    }
}

pub struct StubModeration;
#[async_trait]
impl ManageModerationUseCase for StubModeration {
    async fn list_actions(
        &self,
        _: Option<&str>,
        _: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        unimplemented!()
    }
    async fn log_action(&self, _: LogModerationCommand) -> Result<ModerationAction, DomainError> {
        unimplemented!()
    }
    async fn get_history(&self, _: &str, _: &str) -> Result<UserModerationHistory, DomainError> {
        unimplemented!()
    }
    async fn list_bans(
        &self,
        _: Option<&str>,
        _: i64,
        _: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        unimplemented!()
    }
    async fn delete_bans_for_user(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn delete_action(&self, _: Uuid) -> Result<bool, DomainError> {
        unimplemented!()
    }
}

pub struct StubStats;
#[async_trait]
impl ManageStatsUseCase for StubStats {
    async fn record_messages(
        &self,
        _: manage_stats::RecordMessagesCommand,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn record_voice(&self, _: manage_stats::RecordVoiceCommand) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn get_user_stats(&self, _: &str, _: &str) -> Result<Option<UserStats>, DomainError> {
        unimplemented!()
    }
    async fn get_guild_overview(&self, _: &str) -> Result<GuildStatsOverview, DomainError> {
        unimplemented!()
    }
    async fn get_leaderboard(&self, _: &str, _: u32) -> Result<Vec<UserStats>, DomainError> {
        unimplemented!()
    }
    async fn get_dashboard_stats(&self) -> Result<DashboardStats, DomainError> {
        unimplemented!()
    }
    async fn get_guild_voice_stats(
        &self,
        _: &str,
        _: u32,
        _: u32,
    ) -> Result<GuildVoiceStats, DomainError> {
        unimplemented!()
    }
}

pub struct StubWatchedUsers;
#[async_trait]
impl ManageWatchedUsersUseCase for StubWatchedUsers {
    async fn list_watched_users(
        &self,
        _: Option<&str>,
        _: i64,
        _: i64,
    ) -> Result<Vec<WatchedUser>, DomainError> {
        unimplemented!()
    }
    async fn get_user_dossier(
        &self,
        _: &str,
        _: &str,
    ) -> Result<manage_watched_users::UserDossier, DomainError> {
        unimplemented!()
    }
    async fn add_manual_watch(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn remove_manual_watch(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
}

pub struct StubAuditLogs;
#[async_trait]
impl ManageAuditLogsUseCase for StubAuditLogs {
    async fn create(
        &self,
        cmd: manage_audit_logs::CreateAuditLogCommand,
    ) -> Result<AuditLog, DomainError> {
        Ok(AuditLog {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            event_type: cmd.event_type,
            actor_id: cmd.actor_id,
            actor_name: cmd.actor_name,
            target_id: cmd.target_id,
            target_name: cmd.target_name,
            channel_id: cmd.channel_id,
            channel_name: cmd.channel_name,
            details: cmd.details,
            created_at: chrono::Utc::now(),
        })
    }
    async fn list(
        &self,
        _: Option<&str>,
        _: manage_audit_logs::AuditLogFilters,
    ) -> Result<Vec<AuditLog>, DomainError> {
        unimplemented!()
    }
    async fn count(
        &self,
        _: Option<&str>,
        _: &manage_audit_logs::AuditLogFilters,
    ) -> Result<i64, DomainError> {
        Ok(0)
    }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, DomainError> {
        unimplemented!()
    }
}

/// Stub commun aux use cases de la vie communautaire (planning, annonces de
/// recherche de joueurs, sondages, membre du mois, nouvelles).
///
/// Aucun test de ce fichier n'exerce ces routes. Plutot que cinq stubs qui
/// renverraient des listes vides — ce qui ferait passer silencieusement un
/// test mal cable —, chaque methode remonte `NotImplemented` : si une de ces
/// routes est touchee un jour, l'erreur le dit.
pub struct StubCommunityLife;

fn pas_cable<T>(quoi: &str) -> Result<T, DomainError> {
    Err(DomainError::NotImplemented(format!(
        "{quoi} n'est pas cable dans les tests d'integration"
    )))
}

#[async_trait]
impl sentinel_core::ports::inbound::community::manage_events::ManageEventsUseCase
    for StubCommunityLife
{
    async fn list_window(
        &self,
        _: &str,
        _: sentinel_core::ports::outbound::community::event_repository::EventWindow,
        _: bool,
    ) -> Result<Vec<sentinel_core::domain::entities::community::event::CommunityEvent>, DomainError>
    {
        Ok(vec![])
    }
    async fn get(
        &self,
        _: Uuid,
    ) -> Result<
        sentinel_core::ports::inbound::community::manage_events::EventWithParticipants,
        DomainError,
    > {
        pas_cable("le planning")
    }
    async fn create(
        &self,
        _: sentinel_core::domain::entities::community::event::UpsertEventCommand,
    ) -> Result<sentinel_core::domain::entities::community::event::CommunityEvent, DomainError>
    {
        pas_cable("le planning")
    }
    async fn update(
        &self,
        _: Uuid,
        _: sentinel_core::domain::entities::community::event::UpsertEventCommand,
    ) -> Result<sentinel_core::domain::entities::community::event::CommunityEvent, DomainError>
    {
        pas_cable("le planning")
    }
    async fn delete(&self, _: Uuid) -> Result<(), DomainError> {
        pas_cable("le planning")
    }
    async fn join(
        &self,
        _: Uuid,
        _: &str,
        _: &str,
        _: sentinel_core::domain::entities::community::event::EventAnswer,
    ) -> Result<(), DomainError> {
        pas_cable("le planning")
    }
    async fn leave(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        pas_cable("le planning")
    }
}

#[async_trait]
impl sentinel_core::ports::inbound::community::manage_lfg::ManageLfgUseCase for StubCommunityLife {
    async fn list(
        &self,
        _: &str,
        _: bool,
        _: i64,
    ) -> Result<Vec<sentinel_core::domain::entities::community::lfg::LfgPost>, DomainError> {
        Ok(vec![])
    }
    async fn get(
        &self,
        _: Uuid,
    ) -> Result<sentinel_core::domain::entities::community::lfg::LfgPost, DomainError> {
        pas_cable("les annonces de joueurs")
    }
    async fn create(
        &self,
        _: sentinel_core::domain::entities::community::lfg::UpsertLfgCommand,
    ) -> Result<sentinel_core::domain::entities::community::lfg::LfgPost, DomainError> {
        pas_cable("les annonces de joueurs")
    }
    async fn close(&self, _: Uuid, _: &str, _: bool) -> Result<(), DomainError> {
        pas_cable("les annonces de joueurs")
    }
    async fn delete(&self, _: Uuid, _: &str, _: bool) -> Result<(), DomainError> {
        pas_cable("les annonces de joueurs")
    }
    async fn join(
        &self,
        _: Uuid,
        _: &str,
        _: &str,
    ) -> Result<sentinel_core::domain::entities::community::lfg::LfgPost, DomainError> {
        pas_cable("les annonces de joueurs")
    }
    async fn leave(
        &self,
        _: Uuid,
        _: &str,
    ) -> Result<sentinel_core::domain::entities::community::lfg::LfgPost, DomainError> {
        pas_cable("les annonces de joueurs")
    }
}

#[async_trait]
impl sentinel_core::ports::inbound::community::manage_polls::ManagePollsUseCase
    for StubCommunityLife
{
    async fn list(
        &self,
        _: &str,
        _: bool,
        _: i64,
    ) -> Result<Vec<sentinel_core::domain::entities::community::poll::Poll>, DomainError> {
        Ok(vec![])
    }
    async fn get(
        &self,
        _: Uuid,
        _: Option<&str>,
    ) -> Result<sentinel_core::ports::inbound::community::manage_polls::PollWithVote, DomainError>
    {
        pas_cable("les sondages")
    }
    async fn create(
        &self,
        _: sentinel_core::domain::entities::community::poll::UpsertPollCommand,
    ) -> Result<sentinel_core::domain::entities::community::poll::Poll, DomainError> {
        pas_cable("les sondages")
    }
    async fn close(&self, _: Uuid) -> Result<(), DomainError> {
        pas_cable("les sondages")
    }
    async fn delete(&self, _: Uuid) -> Result<(), DomainError> {
        pas_cable("les sondages")
    }
    async fn vote(
        &self,
        _: Uuid,
        _: Uuid,
        _: &str,
    ) -> Result<sentinel_core::domain::entities::community::poll::Poll, DomainError> {
        pas_cable("les sondages")
    }
}

#[async_trait]
impl sentinel_core::ports::inbound::community::manage_spotlight::ManageSpotlightUseCase
    for StubCommunityLife
{
    async fn current(
        &self,
        _: &str,
        _: Option<&str>,
    ) -> Result<Option<sentinel_core::domain::entities::community::spotlight::Spotlight>, DomainError>
    {
        Ok(None)
    }
    async fn list(
        &self,
        _: &str,
        _: i64,
    ) -> Result<Vec<sentinel_core::domain::entities::community::spotlight::Spotlight>, DomainError>
    {
        Ok(vec![])
    }
    async fn designate(
        &self,
        _: sentinel_core::domain::entities::community::spotlight::UpsertSpotlightCommand,
    ) -> Result<sentinel_core::domain::entities::community::spotlight::Spotlight, DomainError> {
        pas_cable("le membre du mois")
    }
    async fn delete(&self, _: Uuid) -> Result<(), DomainError> {
        pas_cable("le membre du mois")
    }
}

#[async_trait]
impl sentinel_core::ports::inbound::community::read_presence::ReadPresenceUseCase
    for StubCommunityLife
{
    // Vide plutôt que `NotImplemented` : c'est la reponse NORMALE quand
    // personne n'est en vocal, et la page doit s'afficher sans presence.
    async fn voice(
        &self,
        _: &str,
    ) -> Result<
        Option<sentinel_core::domain::entities::community::presence::VoicePresence>,
        DomainError,
    > {
        Ok(None)
    }

    async fn text_activity(
        &self,
        _: &str,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::community::presence::TextChannelActivity>,
        DomainError,
    > {
        Ok(vec![])
    }
}

#[async_trait]
impl sentinel_core::ports::inbound::community::manage_news::ManageNewsUseCase
    for StubCommunityLife
{
    async fn list(
        &self,
        _: &str,
        _: bool,
        _: i64,
    ) -> Result<Vec<sentinel_core::domain::entities::community::news::NewsPost>, DomainError> {
        Ok(vec![])
    }
    async fn get(
        &self,
        _: Uuid,
    ) -> Result<sentinel_core::domain::entities::community::news::NewsPost, DomainError> {
        pas_cable("les nouvelles du site")
    }
    async fn create(
        &self,
        _: sentinel_core::domain::entities::community::news::UpsertNewsCommand,
    ) -> Result<sentinel_core::domain::entities::community::news::NewsPost, DomainError> {
        pas_cable("les nouvelles du site")
    }
    async fn update(
        &self,
        _: Uuid,
        _: sentinel_core::domain::entities::community::news::UpsertNewsCommand,
    ) -> Result<sentinel_core::domain::entities::community::news::NewsPost, DomainError> {
        pas_cable("les nouvelles du site")
    }
    async fn delete(&self, _: Uuid) -> Result<(), DomainError> {
        pas_cable("les nouvelles du site")
    }
}

pub struct StubAuditEventCounter;
#[async_trait]
impl sentinel_core::ports::outbound::audit::audit_event_counter::AuditEventCounter
    for StubAuditEventCounter
{
    async fn count_by_event_type(
        &self,
        _guild_id: &str,
        _days: u32,
    ) -> Result<Vec<(String, u64)>, DomainError> {
        Ok(Vec::new())
    }
}

pub struct StubSnapshots;
#[async_trait]
impl sentinel_core::ports::inbound::audit::manage_snapshots::ManageSnapshotsUseCase
    for StubSnapshots
{
    async fn snapshot_daily_all(
        &self,
    ) -> Result<sentinel_core::domain::entities::audit::snapshot::JobReport, DomainError> {
        Ok(sentinel_core::domain::entities::audit::snapshot::JobReport::ok(0, 0))
    }
    async fn snapshot_hourly_all(
        &self,
    ) -> Result<sentinel_core::domain::entities::audit::snapshot::JobReport, DomainError> {
        Ok(sentinel_core::domain::entities::audit::snapshot::JobReport::ok(0, 0))
    }
    async fn retention_cleanup_all(
        &self,
    ) -> Result<sentinel_core::domain::entities::audit::snapshot::JobReport, DomainError> {
        Ok(sentinel_core::domain::entities::audit::snapshot::JobReport::ok(0, 0))
    }
    async fn plan_top_publications(
        &self,
    ) -> Result<sentinel_core::domain::entities::audit::snapshot::TopPublishPlan, DomainError> {
        Ok(
            sentinel_core::domain::entities::audit::snapshot::TopPublishPlan {
                publications: Vec::new(),
                skipped: 0,
            },
        )
    }
    async fn mark_top_published(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}



pub struct StubAnnouncements;
#[async_trait]
impl sentinel_core::ports::inbound::community::manage_announcements::ManageAnnouncementsUseCase
    for StubAnnouncements
{
    async fn create(
        &self,
        _: sentinel_core::ports::inbound::community::manage_announcements::CreateAnnouncementCommand,
    ) -> Result<
        sentinel_core::domain::entities::community::announcement::ScheduledAnnouncement,
        DomainError,
    > {
        unimplemented!()
    }
    async fn update(
        &self,
        _: sentinel_core::ports::inbound::community::manage_announcements::UpdateAnnouncementCommand,
    ) -> Result<
        sentinel_core::domain::entities::community::announcement::ScheduledAnnouncement,
        DomainError,
    > {
        unimplemented!()
    }
    async fn delete(&self, _: uuid::Uuid) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn get(
        &self,
        _: uuid::Uuid,
    ) -> Result<
        sentinel_core::domain::entities::community::announcement::ScheduledAnnouncement,
        DomainError,
    > {
        unimplemented!()
    }
    async fn list_by_guild(
        &self,
        _: &str,
    ) -> Result<
        Vec<sentinel_core::domain::entities::community::announcement::ScheduledAnnouncement>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn toggle(&self, _: uuid::Uuid, _: bool) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn fetch_due_and_prepare(
        &self,
        _: chrono::DateTime<chrono::Utc>,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::ports::inbound::community::manage_announcements::RenderedAnnouncement>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn record_run_result(
        &self,
        _: uuid::Uuid,
        _: Vec<sentinel_core::domain::entities::community::announcement::ChannelPostResult>,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn preview(
        &self,
        _: uuid::Uuid,
    ) -> Result<
        sentinel_core::ports::inbound::community::manage_announcements::RenderedAnnouncement,
        DomainError,
    > {
        unimplemented!()
    }
    async fn list_runs(
        &self,
        _: uuid::Uuid,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::community::announcement::AnnouncementRun>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn record_button_interaction(
        &self,
        _: uuid::Uuid,
        _: Option<uuid::Uuid>,
        _: String,
        _: Option<String>,
        _: String,
        _: Option<String>,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn list_button_interactions(
        &self,
        _: uuid::Uuid,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::community::announcement::ButtonInteraction>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn retention_cleanup_all(
        &self,
    ) -> Result<
        sentinel_core::ports::inbound::community::manage_announcements::RetentionCleanupSummary,
        DomainError,
    > {
        Ok(
            sentinel_core::ports::inbound::community::manage_announcements::RetentionCleanupSummary {
                guilds_processed: 0,
                guilds_skipped: 0,
                rows_deleted: 0,
            },
        )
    }
}

pub struct StubEmbeds;
#[async_trait]
impl sentinel_core::ports::inbound::community::manage_embeds::ManageEmbedsUseCase for StubEmbeds {
    async fn create(
        &self,
        _: &str,
        _: &str,
        _: sentinel_core::ports::inbound::community::manage_embeds::EmbedInput,
    ) -> Result<sentinel_core::domain::entities::community::embed::Embed, DomainError> {
        unimplemented!()
    }
    async fn update(
        &self,
        _: uuid::Uuid,
        _: sentinel_core::ports::inbound::community::manage_embeds::EmbedInput,
    ) -> Result<sentinel_core::domain::entities::community::embed::Embed, DomainError> {
        unimplemented!()
    }
    async fn delete(&self, _: uuid::Uuid) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn get(
        &self,
        _: uuid::Uuid,
    ) -> Result<sentinel_core::domain::entities::community::embed::Embed, DomainError> {
        unimplemented!()
    }
    async fn list_by_guild(
        &self,
        _: &str,
    ) -> Result<Vec<sentinel_core::domain::entities::community::embed::Embed>, DomainError> {
        unimplemented!()
    }
    async fn prepare_post(
        &self,
        _: uuid::Uuid,
        _: &str,
    ) -> Result<sentinel_core::domain::entities::community::embed::RenderedEmbedPost, DomainError>
    {
        unimplemented!()
    }
    async fn prepare_edit(
        &self,
        _: uuid::Uuid,
    ) -> Result<sentinel_core::domain::entities::community::embed::RenderedEmbedPost, DomainError>
    {
        unimplemented!()
    }
    async fn record_posted(&self, _: uuid::Uuid, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
}

pub struct StubIdeas;
#[async_trait]
impl sentinel_core::ports::inbound::community::manage_ideas::ManageIdeasUseCase for StubIdeas {
    async fn list(
        &self,
        _: sentinel_core::ports::outbound::community::idea_repository::IdeaFilters<'_>,
        _: i64,
        _: i64,
    ) -> Result<Vec<sentinel_core::domain::entities::community::idea::Idea>, DomainError> {
        unimplemented!()
    }
    async fn get(
        &self,
        _: uuid::Uuid,
    ) -> Result<sentinel_core::domain::entities::community::idea::Idea, DomainError> {
        unimplemented!()
    }
    async fn get_detail(
        &self,
        _: uuid::Uuid,
    ) -> Result<sentinel_core::domain::entities::community::idea::IdeaDetail, DomainError> {
        unimplemented!()
    }
    async fn get_by_channel(
        &self,
        _: &str,
    ) -> Result<Option<sentinel_core::domain::entities::community::idea::Idea>, DomainError> {
        unimplemented!()
    }
    async fn create(
        &self,
        _: sentinel_core::ports::inbound::community::manage_ideas::CreateIdeaCommand,
    ) -> Result<sentinel_core::domain::entities::community::idea::Idea, DomainError> {
        unimplemented!()
    }
    async fn decide(
        &self,
        _: sentinel_core::ports::inbound::community::manage_ideas::DecideIdeaCommand,
    ) -> Result<sentinel_core::domain::entities::community::idea::Idea, DomainError> {
        unimplemented!()
    }
    async fn set_channel(
        &self,
        _: uuid::Uuid,
        _: Option<&str>,
    ) -> Result<sentinel_core::domain::entities::community::idea::Idea, DomainError> {
        unimplemented!()
    }
    async fn add_message(
        &self,
        _: sentinel_core::ports::inbound::community::manage_ideas::AddIdeaMessageCommand,
    ) -> Result<sentinel_core::domain::entities::community::idea::IdeaMessage, DomainError> {
        unimplemented!()
    }
    async fn delete(&self, _: uuid::Uuid) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn count_open_by_author(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        unimplemented!()
    }
}

pub struct StubConfessions;
#[async_trait]
impl sentinel_core::ports::inbound::community::manage_confessions::ManageConfessionsUseCase
    for StubConfessions
{
    async fn create(
        &self,
        _: sentinel_core::ports::inbound::community::manage_confessions::CreateConfessionCommand,
    ) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError>
    {
        unimplemented!()
    }
    async fn update_message_refs(
        &self,
        _: uuid::Uuid,
        _: String,
        _: String,
        _: Option<String>,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn edit_content(
        &self,
        _: uuid::Uuid,
        _: &str,
        _: String,
    ) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError>
    {
        unimplemented!()
    }
    async fn delete(
        &self,
        _: uuid::Uuid,
        _: String,
        _: Option<String>,
    ) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError>
    {
        unimplemented!()
    }
    async fn get(
        &self,
        _: uuid::Uuid,
    ) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError>
    {
        unimplemented!()
    }
    async fn get_by_message_id(
        &self,
        _: &str,
    ) -> Result<
        Option<sentinel_core::domain::entities::community::confession::Confession>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn get_by_public_number(
        &self,
        _: &str,
        _: i32,
    ) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError>
    {
        unimplemented!()
    }
    async fn list(
        &self,
        _: &str,
        _: i64,
        _: bool,
    ) -> Result<Vec<sentinel_core::domain::entities::community::confession::Confession>, DomainError>
    {
        unimplemented!()
    }
    async fn create_reply(
        &self,
        _: sentinel_core::ports::inbound::community::manage_confessions::CreateReplyCommand,
    ) -> Result<sentinel_core::domain::entities::community::confession::ConfessionReply, DomainError>
    {
        unimplemented!()
    }
    async fn update_reply_message_id(&self, _: uuid::Uuid, _: String) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn delete_reply(
        &self,
        _: uuid::Uuid,
        _: String,
    ) -> Result<sentinel_core::domain::entities::community::confession::ConfessionReply, DomainError>
    {
        unimplemented!()
    }
    async fn list_replies(
        &self,
        _: uuid::Uuid,
    ) -> Result<
        Vec<sentinel_core::domain::entities::community::confession::ConfessionReply>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn get_reply_parent_guild(&self, _: uuid::Uuid) -> Result<String, DomainError> {
        unimplemented!()
    }
    async fn create_report(
        &self,
        _: sentinel_core::ports::inbound::community::manage_confessions::CreateReportCommand,
    ) -> Result<sentinel_core::domain::entities::community::confession::ConfessionReport, DomainError>
    {
        unimplemented!()
    }
    async fn get_report_guild(&self, _: uuid::Uuid) -> Result<String, DomainError> {
        unimplemented!()
    }
    async fn list_reports(
        &self,
        _: &str,
        _: Option<sentinel_core::domain::entities::community::confession::ReportStatus>,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::community::confession::ConfessionReport>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn resolve_report(
        &self,
        _: uuid::Uuid,
        _: sentinel_core::domain::entities::community::confession::ReportStatus,
        _: String,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn get_config(
        &self,
        _: &str,
    ) -> Result<sentinel_core::domain::entities::community::confession::ConfessionConfig, DomainError>
    {
        unimplemented!()
    }
    async fn save_config(
        &self,
        _: sentinel_core::domain::entities::community::confession::ConfessionConfig,
    ) -> Result<sentinel_core::domain::entities::community::confession::ConfessionConfig, DomainError>
    {
        unimplemented!()
    }
}











// ══════════════════════════════════════════════════════════
// Stub Repositories (outbound)
// ══════════════════════════════════════════════════════════

pub struct StubAnalyticsRepo;
#[async_trait]
impl AnalyticsRepository for StubAnalyticsRepo {
    async fn get_heatmap(
        &self,
        _: Option<&str>,
        _: i32,
    ) -> Result<Vec<HourlyActivity>, DomainError> {
        unimplemented!()
    }
    async fn get_action_distribution(
        &self,
        _: Option<&str>,
        _: i32,
    ) -> Result<Vec<ActionDistribution>, DomainError> {
        unimplemented!()
    }
    async fn get_top_infractors(
        &self,
        _: Option<&str>,
        _: i32,
        _: i64,
        _: i64,
    ) -> Result<Vec<TopInfractor>, DomainError> {
        unimplemented!()
    }
    async fn get_moderation_trend(
        &self,
        _: Option<&str>,
        _: i32,
    ) -> Result<Vec<ModerationTrend>, DomainError> {
        unimplemented!()
    }
    async fn get_peak_hours(
        &self,
        _: Option<&str>,
        _: i32,
    ) -> Result<Vec<PeakActivity>, DomainError> {
        unimplemented!()
    }
    async fn record_hourly(&self, _: &str, _: i16, _: i64, _: i32) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn reset_activity(&self, _: &str) -> Result<u64, DomainError> {
        unimplemented!()
    }
}

pub struct StubDailyActivityRepo;
#[async_trait]
impl DailyActivityRepository for StubDailyActivityRepo {
    async fn get_activity(
        &self,
        _: Option<&str>,
        _: i32,
    ) -> Result<Vec<DailyActivity>, DomainError> {
        unimplemented!()
    }
    async fn record_daily_snapshot(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
}

pub struct StubLogRepo;
#[async_trait]
impl LogRepository for StubLogRepo {
    async fn save(&self, _: &LogEntry) -> Result<(), DomainError> {
        Ok(())
    }
    async fn find_all(&self, _: i64) -> Result<Vec<LogEntry>, DomainError> {
        Ok(vec![])
    }
    async fn find_filtered(
        &self,
        _: Option<&str>,
        _: Option<&str>,
        _: Option<&str>,
        _: i64,
    ) -> Result<Vec<LogEntry>, DomainError> {
        Ok(vec![])
    }
    async fn delete_by_category(&self, _: &str) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn delete_older_than_days(&self, _: i32) -> Result<u64, DomainError> {
        Ok(0)
    }
}

pub struct StubSystemLogs;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_system_logs::ManageSystemLogsUseCase
    for StubSystemLogs
{
    async fn list_logs(
        &self,
        _: sentinel_core::ports::inbound::system::manage_system_logs::SystemLogFilters,
    ) -> Result<Vec<LogEntry>, DomainError> {
        Ok(vec![])
    }
    async fn purge_category(&self, _: &str) -> Result<u64, DomainError> {
        Ok(0)
    }
}

pub struct StubGuildRepo;
#[async_trait]
impl GuildRepository for StubGuildRepo {
    async fn upsert(&self, _: &Guild) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn find_all(&self) -> Result<Vec<Guild>, DomainError> {
        unimplemented!()
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<Guild>, DomainError> {
        unimplemented!()
    }
    async fn delete(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn delete_absent(&self, _: &[String]) -> Result<u64, DomainError> {
        unimplemented!()
    }
}

pub struct StubBotConfigRepo;
#[async_trait]
impl BotConfigRepository for StubBotConfigRepo {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
        unimplemented!()
    }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        unimplemented!()
    }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
}

pub struct StubDiscordRoleRepo;
#[async_trait]
impl DiscordRoleRepository for StubDiscordRoleRepo {
    async fn sync_roles(&self, _: &str, _: Vec<DiscordRole>) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn find_by_guild(&self, _: &str) -> Result<Vec<DiscordRole>, DomainError> {
        unimplemented!()
    }
    async fn find_by_id(&self, _: &str, _: &str) -> Result<Option<DiscordRole>, DomainError> {
        unimplemented!()
    }
}



// ── Stubs pour les nouveaux repos ──

pub struct StubUserActivityRepo;
#[async_trait]
impl UserActivityRepository for StubUserActivityRepo {
    async fn create(&self, _: &UserActivity) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: i64,
        _: i64,
    ) -> Result<Vec<UserActivity>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubWelcomeConfigRepo;
#[async_trait]
impl WelcomeConfigRepository for StubWelcomeConfigRepo {
    async fn get_config(&self, guild_id: &str) -> Result<WelcomeConfigData, DomainError> {
        Ok(WelcomeConfigData {
            guild_id: guild_id.into(),
            welcome_enabled: true,
            welcome_channel_id: None,
            welcome_message: String::new(),
            welcome_embed_color: "3498db".into(),
            welcome_dm_enabled: false,
            welcome_dm_message: String::new(),
            leave_enabled: false,
            leave_channel_id: None,
            leave_message: String::new(),
            rules_enabled: false,
            rules_channel_id: None,
            rules_message: String::new(),
            rules_role_id: None,
            rules_button_label: String::new(),
            age_check_enabled: false,
            age_minimum: 0,
            unverified_role_id: None,
            age_modal_question: String::new(),
            age_ban_message: String::new(),
            age_min: 5,
            age_max: 120,
            age_ban_days_per_year: 365,
            age_ban_log_channel_id: None,
            leave_embed_color: "e74c3c".into(),
            rules_embed_color: "5865f2".into(),
            counter_enabled: false,
            counter_channel_id: None,
            counter_format: String::new(),
            voice_counter_enabled: false,
            voice_counter_channel_id: None,
            voice_counter_format: String::new(),
            anniversary_enabled: false,
            anniversary_channel_id: None,
            anniversary_message: String::new(),
            rejoin_message: String::new(),
            welcome_title: String::new(),
            welcome_image_url: String::new(),
            welcome_footer_text: String::new(),
            rejoin_title: String::new(),
            rejoin_image_url: String::new(),
            rejoin_footer_text: String::new(),
            leave_title: String::new(),
            leave_image_url: String::new(),
            leave_footer_text: String::new(),
            anniversary_title: String::new(),
            anniversary_image_url: String::new(),
            anniversary_footer_text: String::new(),
        })
    }
    async fn save_config(
        &self,
        _: &str,
        d: &WelcomeConfigData,
    ) -> Result<WelcomeConfigData, DomainError> {
        Ok(d.clone())
    }
}

pub struct StubGuildResetRepo;
#[async_trait]
impl sentinel_core::ports::outbound::system::guild_reset_repository::GuildResetRepository
    for StubGuildResetRepo
{
    async fn guild_name(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
    async fn collect_discord_context(
        &self,
        _: &str,
    ) -> Result<
        sentinel_core::ports::outbound::system::guild_reset_repository::ResetDiscordContext,
        DomainError,
    > {
        Ok(Default::default())
    }
    async fn wipe_guild(&self, _: &str) -> Result<Vec<(String, u64)>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubAutomodReviewRepo;
#[async_trait]
impl sentinel_core::ports::outbound::moderation::automod_review_repository::AutomodReviewRepository
    for StubAutomodReviewRepo
{
    async fn create(
        &self,
        _: sentinel_core::domain::entities::moderation::review::automod::NewAutomodReview,
    ) -> Result<
        sentinel_core::domain::entities::moderation::review::automod::AutomodReview,
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn fp_terminal_reviews(
        &self,
        _: &str,
        _: i64,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::FpTerminalReview>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn create_or_merge(
        &self,
        _: sentinel_core::domain::entities::moderation::review::automod::NewAutomodReview,
        _: bool,
        _: i64,
    ) -> Result<
        (
            sentinel_core::domain::entities::moderation::review::automod::AutomodReview,
            bool,
        ),
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn expire_stale_decided(
        &self,
        _: i64,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::ExpiredReviewCard>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn find_discussion(
        &self,
        _: Uuid,
    ) -> Result<
        Option<sentinel_core::domain::entities::moderation::review::automod::DiscussionChannel>,
        DomainError,
    > {
        Ok(None)
    }
    async fn create_discussion(
        &self,
        _: sentinel_core::domain::entities::moderation::review::automod::NewDiscussionChannel,
    ) -> Result<
        (
            sentinel_core::domain::entities::moderation::review::automod::DiscussionChannel,
            bool,
        ),
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn delete_discussion(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn append_discussion_messages(
        &self,
        _: &[sentinel_core::domain::entities::moderation::review::automod::DiscussionMessage],
    ) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn list_discussion_messages(
        &self,
        _: Uuid,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::DiscussionMessage>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn expire_review_cards(
        &self,
        _: i64,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::ExpiredReviewCard>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn upsert_vote(&self, _: Uuid, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_votes(
        &self,
        _: Uuid,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::ReviewVote>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn decide(
        &self,
        _: Uuid,
        _: &str,
        _: bool,
    ) -> Result<
        sentinel_core::domain::entities::moderation::review::automod::AutomodReview,
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn list_expired_voting(
        &self,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::AutomodReview>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn get(
        &self,
        _: Uuid,
    ) -> Result<
        Option<sentinel_core::domain::entities::moderation::review::automod::AutomodReview>,
        DomainError,
    > {
        Ok(None)
    }
    async fn find_by_message_id(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        Option<sentinel_core::domain::entities::moderation::review::automod::AutomodReview>,
        DomainError,
    > {
        Ok(None)
    }
    async fn list_pending(
        &self,
        _: &str,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::AutomodReview>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn list_recent(
        &self,
        _: &str,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::AutomodReview>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn resolve(
        &self,
        _: Uuid,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<
        sentinel_core::domain::entities::moderation::review::automod::AutomodReview,
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn close_ignored(
        &self,
        _: Uuid,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<
        sentinel_core::domain::entities::moderation::review::automod::AutomodReview,
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn reopen(
        &self,
        _: Uuid,
        _: i64,
    ) -> Result<
        sentinel_core::domain::entities::moderation::review::automod::AutomodReview,
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
}

pub struct StubDiscordActionMessageRepo;
#[async_trait]
impl sentinel_core::ports::outbound::audit::discord_action_message_repository::DiscordActionMessageRepository for StubDiscordActionMessageRepo {
    async fn register(&self, _: sentinel_core::domain::entities::audit::discord_action_message::NewDiscordActionMessage) -> Result<(), DomainError> { Ok(()) }
    async fn list_for_action(&self, _: Uuid) -> Result<Vec<sentinel_core::domain::entities::audit::discord_action_message::DiscordActionMessage>, DomainError> { Ok(vec![]) }
}

pub struct StubExportUC;
#[async_trait]
impl sentinel_core::application::system::export_service::ExecuteExportUseCase for StubExportUC {
    async fn execute(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<sentinel_core::application::system::export_service::ExportResult, DomainError> {
        Ok(
            sentinel_core::application::system::export_service::ExportResult {
                data: String::new(),
                row_count: 0,
            },
        )
    }
}

pub struct StubExportJobsUC;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_export_jobs::ManageExportJobsUseCase
    for StubExportJobsUC
{
    async fn enqueue(
        &self,
        _: sentinel_core::ports::outbound::system::export_job_repository::NewExportJob,
    ) -> Result<Uuid, DomainError> {
        Ok(Uuid::new_v4())
    }
    async fn get(
        &self,
        _: Uuid,
    ) -> Result<
        Option<sentinel_core::ports::outbound::system::export_job_repository::ExportJobRecord>,
        DomainError,
    > {
        Ok(None)
    }
}

pub struct StubEvidenceRepo;
#[async_trait]
impl EvidenceRepository for StubEvidenceRepo {
    async fn add(
        &self,
        _: Uuid,
        _: &str,
        _: Option<&str>,
        _: &str,
        _: &str,
    ) -> Result<EvidenceEntry, DomainError> {
        Err(DomainError::Internal("stub".into()))
    }
    async fn list(&self, _: Uuid) -> Result<Vec<EvidenceEntry>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubReviewRepo;
#[async_trait]
impl ReviewRepository for StubReviewRepo {
    async fn add(
        &self,
        _: Uuid,
        _: &str,
        _: &str,
        _: &str,
        _: Option<&str>,
    ) -> Result<ReviewEntry, DomainError> {
        Err(DomainError::Internal("stub".into()))
    }
    async fn list_pending(&self, _: &str) -> Result<Vec<ReviewEntry>, DomainError> {
        Ok(vec![])
    }
    async fn resolve(
        &self,
        _: Uuid,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: &str,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn get_guild_id(&self, _: Uuid) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
}

pub struct StubModstatsRepo;
#[async_trait]
impl ModstatsRepository for StubModstatsRepo {
    async fn top_moderators(
        &self,
        _: &str,
        _: i32,
        _: i64,
    ) -> Result<Vec<ModeratorStat>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubSponsorshipRepo;
#[async_trait]
impl SponsorshipRepository for StubSponsorshipRepo {
    async fn create(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list(&self, _: &str) -> Result<Vec<Sponsorship>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubTempRoleRepo;
#[async_trait]
impl TempRoleRepository for StubTempRoleRepo {
    async fn create(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_active(&self, _: &str) -> Result<Vec<TempRole>, DomainError> {
        Ok(vec![])
    }
    async fn delete(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubPendingActionRepo;
#[async_trait]
impl PendingActionRepository for StubPendingActionRepo {
    async fn create(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<i64>,
    ) -> Result<Uuid, DomainError> {
        Ok(Uuid::new_v4())
    }
    async fn list_pending(&self, _: &str) -> Result<Vec<PendingAction>, DomainError> {
        Ok(vec![])
    }
    async fn get_guild_id(&self, _: Uuid) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
    async fn resolve(&self, _: Uuid, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

// ── Stubs Tamagotchi / Systeme securite ──

pub struct StubGuildSnapshots;
#[async_trait]
impl sentinel_core::ports::inbound::guild_backup::manage_snapshots::ManageGuildSnapshotsUseCase
    for StubGuildSnapshots
{
    async fn store_snapshot(
        &self,
        _: sentinel_core::domain::entities::guild_backup::snapshot::GuildSnapshot,
    ) -> Result<
        sentinel_core::ports::inbound::guild_backup::manage_snapshots::SnapshotId,
        DomainError,
    > {
        unimplemented!()
    }
    async fn store_snapshot_with_quota(
        &self,
        _: sentinel_core::domain::entities::guild_backup::snapshot::GuildSnapshot,
        _: u32,
    ) -> Result<
        sentinel_core::ports::inbound::guild_backup::manage_snapshots::SnapshotId,
        DomainError,
    > {
        unimplemented!()
    }
    async fn list_snapshots(
        &self,
        _: &str,
    ) -> Result<
        Vec<sentinel_core::ports::inbound::guild_backup::manage_snapshots::SnapshotSummary>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn get_snapshot(
        &self,
        _: sentinel_core::ports::inbound::guild_backup::manage_snapshots::SnapshotId,
    ) -> Result<sentinel_core::domain::entities::guild_backup::snapshot::GuildSnapshot, DomainError>
    {
        unimplemented!()
    }
    async fn delete_snapshot(
        &self,
        _: sentinel_core::ports::inbound::guild_backup::manage_snapshots::SnapshotId,
    ) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn rename_snapshot(
        &self,
        _: sentinel_core::ports::inbound::guild_backup::manage_snapshots::SnapshotId,
        _: &str,
    ) -> Result<bool, DomainError> {
        unimplemented!()
    }
}

pub struct StubPendingRoleGrants;
#[async_trait]
impl sentinel_core::ports::inbound::guild_backup::manage_pending_role_grants::ManagePendingRoleGrantsUseCase
    for StubPendingRoleGrants
{
    async fn save_grants(
        &self,
        _: &str,
        _: Vec<sentinel_core::domain::entities::guild_backup::pending_role_grant::PendingRoleGrant>,
    ) -> Result<u64, DomainError> {
        unimplemented!()
    }
    async fn take_grant(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<Vec<String>>, DomainError> {
        unimplemented!()
    }
    async fn clear_guild(&self, _: &str) -> Result<u64, DomainError> {
        unimplemented!()
    }
}

pub struct StubIpBans;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_ip_bans::ManageIpBansUseCase for StubIpBans {
    async fn ban(
        &self,
        _: &str,
        _: Option<String>,
        _: &str,
    ) -> Result<sentinel_core::domain::entities::system::ip_ban::BanIpOutcome, DomainError> {
        unimplemented!()
    }
    async fn unban(&self, _: &str, _: Option<String>, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn list_manual_bans(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::system::ip_ban::ManualIpBan>, DomainError>
    {
        unimplemented!()
    }
    async fn fail2ban_status(
        &self,
    ) -> Result<Option<sentinel_core::domain::entities::system::ip_ban::Fail2banStatus>, DomainError>
    {
        unimplemented!()
    }
}

pub struct StubHostProbe;
#[async_trait]
impl sentinel_core::ports::inbound::system::read_host_probe::ReadHostProbeUseCase
    for StubHostProbe
{
    async fn read(
        &self,
        _: sentinel_core::domain::entities::system::host_probe::HostProbe,
    ) -> Result<serde_json::Value, DomainError> {
        unimplemented!()
    }
}

pub struct StubSecurityLogs;
#[async_trait]
impl sentinel_core::ports::inbound::system::read_security_logs::ReadSecurityLogsUseCase
    for StubSecurityLogs
{
    async fn top_ips(
        &self,
        _: sentinel_core::domain::entities::system::security_log::LogWindow,
        _: i64,
    ) -> Result<Vec<sentinel_core::domain::entities::system::security_log::TopIp>, DomainError>
    {
        unimplemented!()
    }
    async fn auth_failures(
        &self,
        _: sentinel_core::domain::entities::system::security_log::LogWindow,
        _: i64,
    ) -> Result<Vec<sentinel_core::domain::entities::system::security_log::AuthFailure>, DomainError>
    {
        unimplemented!()
    }
    async fn traffic_trend(
        &self,
        _: sentinel_core::domain::entities::system::security_log::LogWindow,
        _: i64,
    ) -> Result<sentinel_core::domain::entities::system::security_log::TrafficTrend, DomainError>
    {
        unimplemented!()
    }
}

pub struct StubSecurityAudit;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_security_audit::ManageSecurityAuditUseCase
    for StubSecurityAudit
{
    async fn audit_logs(
        &self,
        _: sentinel_core::domain::entities::system::security_audit::AuditLogFilter,
    ) -> Result<
        Vec<sentinel_core::domain::entities::system::security_audit::AuditLogEntry>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn recent_logins(
        &self,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::system::security_audit::SuccessfulLogin>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn cleanup(
        &self,
        _: sentinel_core::domain::entities::system::security_audit::CleanupOptions,
    ) -> Result<sentinel_core::domain::entities::system::security_audit::CleanupReport, DomainError>
    {
        unimplemented!()
    }
}

pub struct StubTlsCert;
#[async_trait]
impl sentinel_core::ports::inbound::system::read_tls_cert::ReadTlsCertUseCase for StubTlsCert {
    async fn read(
        &self,
    ) -> Result<sentinel_core::domain::entities::system::tls_cert::TlsCertInfo, DomainError> {
        unimplemented!()
    }
}

pub struct StubGeoIp;
#[async_trait]
impl sentinel_core::ports::inbound::system::lookup_geoip::LookupGeoIpUseCase for StubGeoIp {
    async fn lookup(
        &self,
        _: Vec<String>,
    ) -> Result<Vec<sentinel_core::domain::entities::system::geoip::GeoIpEntry>, DomainError> {
        unimplemented!()
    }
}

// ══════════════════════════════════════════════════════════
// Stubs additionnels (champs AppState recents)
// ══════════════════════════════════════════════════════════



pub struct StubEligibility;
#[async_trait]
impl sentinel_core::ports::inbound::community::check_eligibility::CheckEligibilityUseCase
    for StubEligibility
{
    async fn check_role_eligibility(
        &self,
        _: sentinel_core::ports::inbound::community::check_eligibility::CheckRoleEligibilityCommand,
    ) -> Result<
        sentinel_core::domain::entities::community::eligibility::EligibilityDecision,
        DomainError,
    > {
        Ok(sentinel_core::domain::entities::community::eligibility::EligibilityDecision::allow())
    }
    async fn validate_sponsorship(
        &self,
        _: sentinel_core::ports::inbound::community::check_eligibility::ValidateSponsorshipCommand,
    ) -> Result<
        sentinel_core::domain::entities::community::eligibility::EligibilityDecision,
        DomainError,
    > {
        Ok(sentinel_core::domain::entities::community::eligibility::EligibilityDecision::allow())
    }
}

pub struct StubDataset;
#[async_trait]
impl sentinel_core::ports::inbound::ai::manage_dataset::ManageDatasetUseCase for StubDataset {
    async fn collect_message(
        &self,
        _: sentinel_core::ports::outbound::ai::dataset_repository::NewDatasetMessage,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_messages(
        &self,
        _: sentinel_core::ports::inbound::ai::manage_dataset::ListDatasetQuery,
    ) -> Result<sentinel_core::domain::entities::ai::dataset::DatasetPage, DomainError> {
        Ok(sentinel_core::domain::entities::ai::dataset::DatasetPage {
            items: vec![],
            total: 0,
        })
    }
    async fn bulk_delete(
        &self,
        _: sentinel_core::ports::inbound::ai::manage_dataset::BulkDeleteCommand,
    ) -> Result<i64, DomainError> {
        Ok(0)
    }
}

pub struct StubAiJobs;
#[async_trait]
impl sentinel_core::ports::inbound::ai::manage_ai_jobs::ManageAiJobsUseCase for StubAiJobs {
    async fn create_job(
        &self,
        _: sentinel_core::domain::entities::ai::ai_job::NewAiJob,
    ) -> Result<Uuid, DomainError> {
        Ok(Uuid::new_v4())
    }
    async fn get_job(
        &self,
        _: Uuid,
    ) -> Result<sentinel_core::domain::entities::ai::ai_job::AiJob, DomainError> {
        Err(DomainError::NotFound("ai_job stub".into()))
    }
}

pub struct StubOAuth;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_oauth::ManageOAuthUseCase for StubOAuth {
    async fn record_login(
        &self,
        _: sentinel_core::domain::entities::system::oauth::LoginTrace,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn create_session(
        &self,
        _: sentinel_core::domain::entities::system::oauth::NewOAuthSession,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_session(
        &self,
        _: Uuid,
    ) -> Result<Option<sentinel_core::domain::entities::system::oauth::OAuthSession>, DomainError>
    {
        Ok(None)
    }
    async fn touch_session(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update_tokens(
        &self,
        _: sentinel_core::domain::entities::system::oauth::SessionTokenUpdate,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_session(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubQuarantine;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_quarantine::ManageQuarantineUseCase
    for StubQuarantine
{
    async fn quarantine_user(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_active(
        &self,
    ) -> Result<
        Vec<sentinel_core::domain::entities::system::quarantine::ActiveQuarantine>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn lift(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubAlertRules;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_alert_rules::ManageAlertRulesUseCase
    for StubAlertRules
{
    async fn list(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::system::alert_rule::AlertRule>, DomainError>
    {
        Ok(vec![])
    }
    async fn update(
        &self,
        _: &str,
        _: sentinel_core::domain::entities::system::alert_rule::AlertRuleUpdate,
    ) -> Result<sentinel_core::domain::entities::system::alert_rule::AlertRule, DomainError> {
        Err(DomainError::NotFound("regle d'alerte inconnue".into()))
    }
}

pub struct StubSystemProbe;
#[async_trait]
impl sentinel_core::ports::outbound::system::system_probe::SystemProbe for StubSystemProbe {}

pub struct StubDockerHost;
#[async_trait]
impl sentinel_core::ports::outbound::system::docker_host::DockerHost for StubDockerHost {
    async fn version_info(
        &self,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::DockerVersionInfo, DomainError>
    {
        Ok(Default::default())
    }
    async fn disk_usage(
        &self,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::DiskUsage, DomainError> {
        Ok(Default::default())
    }
    async fn list_containers(
        &self,
        _: bool,
    ) -> Result<
        Vec<sentinel_core::domain::entities::system::docker_host::ContainerSummary>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn start_container(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn stop_container(&self, _: &str, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn restart_container(&self, _: &str, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_container(&self, _: &str, _: bool, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn container_logs(&self, _: &str, _: u32, _: bool) -> Result<String, DomainError> {
        Ok(String::new())
    }
    async fn list_images(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::system::docker_host::ImageSummary>, DomainError>
    {
        Ok(vec![])
    }
    async fn remove_image(&self, _: &str, _: bool, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_volumes(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::system::docker_host::VolumeSummary>, DomainError>
    {
        Ok(vec![])
    }
    async fn remove_volume(&self, _: &str, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_networks(
        &self,
    ) -> Result<
        Vec<sentinel_core::domain::entities::system::docker_host::NetworkSummary>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn prune_containers(
        &self,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::PruneOutcome, DomainError>
    {
        Ok(Default::default())
    }
    async fn prune_images(
        &self,
        _: bool,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::PruneOutcome, DomainError>
    {
        Ok(Default::default())
    }
    async fn prune_volumes(
        &self,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::PruneOutcome, DomainError>
    {
        Ok(Default::default())
    }
    async fn prune_networks(
        &self,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::PruneOutcome, DomainError>
    {
        Ok(Default::default())
    }
    async fn prune_build_cache(
        &self,
        _: bool,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::PruneOutcome, DomainError>
    {
        Ok(Default::default())
    }
}

pub struct StubLockdown;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_lockdown::ManageLockdownUseCase
    for StubLockdown
{
    async fn activate(&self, _: &str, _: serde_json::Value, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn deactivate(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubSlowmode;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_slowmode::ManageSlowmodeUseCase
    for StubSlowmode
{
    async fn activate(
        &self,
        _: &str,
        _: serde_json::Value,
        _: i64,
        _: i32,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn deactivate(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubBotPersistence;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_bot_persistence::ManageBotPersistenceUseCase
    for StubBotPersistence
{
    async fn update_streak(
        &self,
        _: &str,
        _: &str,
        _: i32,
        _: i32,
        _: i32,
        _: i32,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubServerEvents;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_server_events::ManageServerEventsUseCase
    for StubServerEvents
{
    async fn record(
        &self,
        _: &str,
        _: Option<&str>,
        _: &str,
        _: Option<&str>,
        _: &str,
        _: serde_json::Value,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list(
        &self,
        _: Option<String>,
        _: Option<String>,
        _: Option<i64>,
    ) -> Result<Vec<sentinel_core::domain::entities::system::server_event::ServerEvent>, DomainError>
    {
        Ok(vec![])
    }
}

pub struct StubMonthlyRanking;
#[async_trait]
impl sentinel_core::ports::inbound::community::manage_monthly_ranking::ManageMonthlyRankingUseCase
    for StubMonthlyRanking
{
    async fn force_ranking(
        &self,
        _: &str,
        _: Option<String>,
    ) -> Result<
        sentinel_core::domain::entities::community::monthly_ranking::MonthlyRankingData,
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn plan_and_baseline(
        &self,
    ) -> Result<
        sentinel_core::domain::entities::community::monthly_ranking::MonthlyPublishPlan,
        DomainError,
    > {
        Ok(Default::default())
    }
    async fn mark_published(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubSursis;
#[async_trait]
impl sentinel_core::ports::inbound::moderation::manage_sursis::ManageSursisUseCase for StubSursis {
    async fn create(
        &self,
        _: sentinel_core::ports::inbound::moderation::manage_sursis::CreateSursisCommand,
    ) -> Result<sentinel_core::domain::entities::moderation::sursis::Sursis, DomainError> {
        Err(DomainError::Internal("stub".into()))
    }
    async fn get(
        &self,
        _: Uuid,
    ) -> Result<Option<sentinel_core::domain::entities::moderation::sursis::Sursis>, DomainError>
    {
        Ok(None)
    }
    async fn resolve(
        &self,
        _: Uuid,
        _: sentinel_core::domain::entities::moderation::sursis::SursisStatus,
    ) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn list_due(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::moderation::sursis::Sursis>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubAdaptiveSlowmodeRepo;
#[async_trait]
impl sentinel_core::ports::outbound::moderation::adaptive_slowmode_repository::AdaptiveSlowmodeRepository
    for StubAdaptiveSlowmodeRepo
{
    async fn mark(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn unmark(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_all(&self) -> Result<Vec<(String, String)>, DomainError> {
        Ok(vec![])
    }
}

// ══════════════════════════════════════════════════════════
// TestAppState builder
// ══════════════════════════════════════════════════════════

struct StubSponsorships;
#[async_trait]
impl sentinel_core::ports::inbound::community::manage_sponsorships::ManageSponsorshipsUseCase
    for StubSponsorships
{
    async fn create_sponsorship(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_sponsorships(&self, _: &str) -> Result<Vec<Sponsorship>, DomainError> {
        Ok(vec![])
    }
    async fn create_temp_role(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_temp_roles(&self, _: &str) -> Result<Vec<TempRole>, DomainError> {
        Ok(vec![])
    }
    async fn delete_temp_role(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

/// Construit un AppState de base avec tous les stubs.
fn base_state() -> AppState {
    // On branche sur le compose de test (6380/5433) pour que les branches
    // redis/sqlx direct des handlers (caches, api_user_guilds, modstats, etc.)
    // soient reellement executees pendant les tests d'integration HTTP.
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6380".to_string());
    let redis_client = redis::Client::open(redis_url).unwrap();
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".to_string()
    });
    let pg_pool = sqlx::PgPool::connect_lazy(&db_url).unwrap();

    // ── Dependances partagees entre l'etat plat et les sous-etats ──
    //
    // Le broadcaster DOIT etre une instance unique : les tests qui verifient
    // qu'un handler a diffuse un evenement s'abonnent a ce canal-la. Deux
    // instances feraient passer des tests sur un canal que personne n'ecoute.
    // Les autres stubs ci-dessous sont hoistes pour la meme raison : un
    // handler migre lit le sous-etat, le test seede l'etat plat.
    let broadcaster = Arc::new(EventBroadcaster::new());
    let bot_config_repo: Arc<
        dyn sentinel_core::ports::outbound::system::bot_config_repository::BotConfigRepository,
    > = Arc::new(StubBotConfigRepo);
    let guild_snapshots_uc: Arc<dyn sentinel_core::ports::inbound::guild_backup::manage_snapshots::ManageGuildSnapshotsUseCase> =
        Arc::new(StubGuildSnapshots);
    let pending_role_grants_uc: Arc<dyn sentinel_core::ports::inbound::guild_backup::manage_pending_role_grants::ManagePendingRoleGrantsUseCase> =
        Arc::new(StubPendingRoleGrants);
    let discord_api: Arc<dyn sentinel_core::ports::outbound::discord_api::DiscordApi> =
        Arc::new(DiscordApiService::new(String::new()));
    let log_repo: Arc<dyn ops_core::ports::outbound::log_repository::LogRepository> =
        Arc::new(StubLogRepo);
    // Partage entre l'etat plat (lu par les middlewares) et `SystemState` :
    // deux listes distinctes laisseraient un test regler l'une et verifier
    // l'autre.
    let superadmin_user_ids: Arc<Vec<String>> = Arc::new(Vec::new());

    // Sous-etats par domaine (cf. `sentinel_api::bootstrap::state`). Seul
    // `guild_backup` a des handlers migres a ce stade ; `ai` et `moderation`
    // sont cables pour que l'etat soit complet, mais encore inutilises.
    let guild_backup = sentinel_api::bootstrap::state::GuildBackupState {
        guild_snapshots_uc: guild_snapshots_uc.clone(),
        pending_role_grants_uc: pending_role_grants_uc.clone(),
        bot_config_repo: bot_config_repo.clone(),
        broadcaster: broadcaster.clone(),
    };
    let ai = sentinel_api::bootstrap::state::AiState {
        analyze_uc: Arc::new(StubAnalyzeMessage),
        analyze_image_uc: Arc::new(StubAnalyzeImage),
        dataset_uc: Arc::new(StubDataset),
        ai_jobs_uc: Arc::new(StubAiJobs),
        inference: Arc::new(
            sentinel_api::adapters::outbound::inference_service::InferenceService::new(None, None),
        ),
        broadcaster: broadcaster.clone(),
    };
    let moderation = sentinel_api::bootstrap::state::ModerationState {
        rules_uc: Arc::new(StubRules),
        infractions_uc: Arc::new(StubInfractions),
        moderation_uc: Arc::new(StubModeration),
        modstats_uc: Arc::new(sentinel_core::application::moderation::read_modstats_service::ReadModstatsService::new(Arc::new(StubModstatsRepo))),
        
        
        
        moderation_copilot_uc: Arc::new(StubModerationCopilot),
        assess_target_risk_uc: Arc::new(
            sentinel_core::application::moderation::assess_target_risk_service::AssessTargetRiskService::new(
                Arc::new(StubBotConfigRepo),
            ),
        ),
        automod_reviews_uc: Arc::new(
            sentinel_core::application::moderation::manage_automod_reviews_service::ManageAutomodReviewsService::new(
                Arc::new(StubAutomodReviewRepo),
            ),
        ),
        automod_adaptive_slowmode_repo: Arc::new(StubAdaptiveSlowmodeRepo),
        sursis_uc: Arc::new(StubSursis),
        // Orchestration reelle : elle ne fait que composer les stubs
        // ci-dessus, un test qui annule une action obtient donc le vrai
        // enchainement (effet Discord inverse puis suppression).
        cancel_action_uc: Arc::new(
            sentinel_core::application::moderation::cancel_action_service::CancelModerationActionService::new(
                Arc::new(StubModeration),
                Arc::new(StubReminders),
                discord_api.clone(),
            ),
        ),
        evidence_repo: Arc::new(StubEvidenceRepo),
        review_repo: Arc::new(StubReviewRepo),
        pending_action_repo: Arc::new(StubPendingActionRepo),
        modstats_repo: Arc::new(StubModstatsRepo),
        broadcaster: broadcaster.clone(),
        discord_api: discord_api.clone(),
        bot_config_repo: bot_config_repo.clone(),
    };

    let audit = sentinel_api::bootstrap::state::AuditState {
        audit_logs_uc: Arc::new(StubAuditLogs),
        watched_users_uc: Arc::new(StubWatchedUsers),
        stats_uc: Arc::new(StubStats),
        detect_anomaly_uc: Arc::new(
            sentinel_core::application::audit::detect_moderation_anomaly_service::DetectModerationAnomalyService::new(
                Arc::new(sentinel_api::adapters::outbound::audit::in_memory_anomaly_counter::InMemoryAnomalyCounter::new(500, 100)),
            ),
        ),
        weekly_report_uc: Arc::new(
            sentinel_core::application::audit::get_weekly_report_service::GetWeeklyReportService::new(
                Arc::new(StubAuditEventCounter),
            ),
        ),
        snapshots_uc: Arc::new(StubSnapshots),
        discord_action_messages_uc: Arc::new(
            sentinel_core::application::audit::manage_discord_action_messages_service::ManageDiscordActionMessagesService::new(
                Arc::new(StubDiscordActionMessageRepo),
            ),
        ),
        security_uc: Arc::new(StubSecurity),
        analytics_repo: Arc::new(StubAnalyticsRepo),
        user_activity_repo: Arc::new(StubUserActivityRepo),
        broadcaster: broadcaster.clone(),
        bot_config_repo: bot_config_repo.clone(),
        redis_client: redis_client.clone(),
        daily_activity_repo: Arc::new(StubDailyActivityRepo),
        discord_api: discord_api.clone(),
    };

    let community = sentinel_api::bootstrap::state::CommunityState {
        events_uc: Arc::new(StubCommunityLife),
        lfg_uc: Arc::new(StubCommunityLife),
        polls_uc: Arc::new(StubCommunityLife),
        spotlight_uc: Arc::new(StubCommunityLife),
        news_uc: Arc::new(StubCommunityLife),
        ideas_uc: Arc::new(StubIdeas),
        confessions_uc: Arc::new(StubConfessions),
        announcements_uc: Arc::new(StubAnnouncements),
        embeds_uc: Arc::new(StubEmbeds),
        
        presence_uc: Arc::new(StubCommunityLife),
        
        
        monthly_ranking_uc: Arc::new(StubMonthlyRanking),
        
        
        welcome_config_uc: Arc::new(
            sentinel_core::application::community::manage_welcome_config_service::ManageWelcomeConfigService::new(
                Arc::new(StubWelcomeConfigRepo),
            ),
        ),
        eligibility_uc: Arc::new(StubEligibility),
        age_check_uc: Arc::new(
            sentinel_core::application::community::evaluate_age_declaration_service::EvaluateAgeDeclarationService::new(
                Arc::new(StubWelcomeConfigRepo),
            ),
        ),
        manage_sponsorships_uc: Arc::new(StubSponsorships),
        daily_activity_repo: Arc::new(StubDailyActivityRepo),
        discord_role_repo: Arc::new(StubDiscordRoleRepo),
        age_ban_repo: Arc::new(
            sentinel_api::adapters::outbound::postgres::community::age_ban_repository::PgAgeBanRepository::new(
                pg_pool.clone(),
            ),
        ),
        sponsorship_repo: Arc::new(StubSponsorshipRepo),
        temp_role_repo: Arc::new(StubTempRoleRepo),
        broadcaster: broadcaster.clone(),
        discord_api: discord_api.clone(),
        bot_config_repo: bot_config_repo.clone(),
        redis_client: redis_client.clone(),
    };

    let system = sentinel_api::bootstrap::state::SystemState {
        tickets_uc: Arc::new(StubTickets),
        system_logs_uc: Arc::new(StubSystemLogs),
        server_events_uc: Arc::new(StubServerEvents),
        reset_guild_uc: Arc::new(
            sentinel_core::application::system::reset_guild_service::ResetGuildService::new(
                Arc::new(StubGuildResetRepo),
            ),
        ),
        bot_persistence_uc: Arc::new(StubBotPersistence),
        alert_rules_uc: Arc::new(StubAlertRules),
        oauth_uc: Arc::new(StubOAuth),
        ip_bans_uc: Arc::new(StubIpBans),
        quarantine_uc: Arc::new(StubQuarantine),
        lockdown_uc: Arc::new(StubLockdown),
        slowmode_uc: Arc::new(StubSlowmode),
        security_logs_uc: Arc::new(StubSecurityLogs),
        security_audit_uc: Arc::new(StubSecurityAudit),
        host_probe_uc: Arc::new(StubHostProbe),
        tls_cert_uc: Arc::new(StubTlsCert),
        geoip_uc: Arc::new(StubGeoIp),
        export_uc: Arc::new(StubExportUC),
        export_jobs_uc: Arc::new(StubExportJobsUC),
        docker_host: Arc::new(StubDockerHost),
        system_probe: Arc::new(StubSystemProbe),
        guild_repo: Arc::new(StubGuildRepo),
        log_repo: log_repo.clone(),
        // Aucun poller Docker ni ban automatique en test.
        container_monitor: None,
        rate_limiter: None,
        broadcaster: broadcaster.clone(),
        discord_api: discord_api.clone(),
        bot_config_repo: bot_config_repo.clone(),
        redis_client: redis_client.clone(),
        discord_oauth_client_id: String::new(),
        discord_oauth_client_secret: String::new(),
        discord_oauth_redirect_uri: String::new(),
        web_front_url: String::new(),
        superadmin_user_ids: superadmin_user_ids.clone(),
        api_key: String::new(),
    };

    AppState {
        audit,
        community,
        system,
        ai,
        moderation,
        guild_backup,
        log_repo: log_repo.clone(),
        bot_config_repo: bot_config_repo.clone(),
        broadcaster: broadcaster.clone(),
        job_client: JobClient::new(redis_client.clone(), "test:jobs".into()),
        discord_api: discord_api.clone(),
        api_key: String::new(),
        // Vide = verrou mono-serveur desactive : les tests d'integration
        // utilisent des identifiants de guilde arbitraires.
        guild_id: String::new(),
        // URL vide : le client se declare non configure et les handlers
        // de jeux repondent « indisponible » au lieu d'appeler dans le vide.
        nexus_games: Arc::new(
            sentinel_api::adapters::outbound::nexus_games::NexusGamesClient::new(
                String::new(),
                String::new(),
            ),
        ),
        discord_bot_token: String::new(),
        pg_pool,
        redis_client,
        cache: None,
        superadmin_user_ids: superadmin_user_ids.clone(),
        metrics_token: String::new(),
    }
}

/// Construit un AppState avec un mock voice channels injecte.
pub fn build_test_state(voice_uc: Arc<dyn ManageVoiceChannelsUseCase>) -> AppState {
    let mut state = base_state();
    state.community.voice_channels_uc = voice_uc;
    state
}

/// Construit un AppState avec un mock tickets injecte.
pub fn build_test_state_tickets(tickets_uc: Arc<dyn ManageTicketsUseCase>) -> AppState {
    let mut state = base_state();
    state.system.tickets_uc = tickets_uc;
    state
}

/// Construit un AppState avec un mock strikes injecte.


/// Construit un AppState avec un mock rules injecte.
pub fn build_test_state_rules(rules_uc: Arc<dyn ManageRulesUseCase>) -> AppState {
    let mut state = base_state();
    state.moderation.rules_uc = rules_uc;
    state
}

/// Construit un AppState avec un mock infractions injecte.
pub fn build_test_state_infractions(infractions_uc: Arc<dyn ManageInfractionsUseCase>) -> AppState {
    let mut state = base_state();
    state.moderation.infractions_uc = infractions_uc;
    state
}

/// Construit un AppState avec un mock audit logs injecte.
pub fn build_test_state_audit_logs(audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>) -> AppState {
    let mut state = base_state();
    state.audit.audit_logs_uc = audit_logs_uc;
    state
}

/// Construit un AppState avec un mock watched users injecte.
pub fn build_test_state_watched_users(
    watched_users_uc: Arc<dyn ManageWatchedUsersUseCase>,
) -> AppState {
    let mut state = base_state();
    state.audit.watched_users_uc = watched_users_uc;
    state
}

/// Construit un AppState avec un mock user activity repository injecte.
pub fn build_test_state_user_activity(
    user_activity_repo: Arc<dyn UserActivityRepository>,
) -> AppState {
    let mut state = base_state();
    state.audit.user_activity_repo = user_activity_repo;
    state
}

/// Construit un AppState avec un mock analyze (text) use case injecte.
pub fn build_test_state_analyze(analyze_uc: Arc<dyn AnalyzeMessageUseCase>) -> AppState {
    let mut state = base_state();
    state.ai.analyze_uc = analyze_uc;
    state
}

/// Construit un AppState avec un mock security use case injecte.
pub fn build_test_state_security(security_uc: Arc<dyn ManageSecurityUseCase>) -> AppState {
    let mut state = base_state();
    state.audit.security_uc = security_uc;
    state
}

/// Construit un AppState avec un mock levels use case injecte.


/// Construit un AppState avec un mock stats use case injecte.
pub fn build_test_state_stats(stats_uc: Arc<dyn ManageStatsUseCase>) -> AppState {
    let mut state = base_state();
    state.audit.stats_uc = stats_uc;
    state
}

/// Construit un AppState avec un mock log repository injecte.
pub fn build_test_state_logs(log_repo: Arc<dyn LogRepository>) -> AppState {
    let mut state = base_state();
    state.log_repo = log_repo;
    state
}

/// Construit un AppState avec un mock guild repository injecte.
pub fn build_test_state_guilds(guild_repo: Arc<dyn GuildRepository>) -> AppState {
    let mut state = base_state();
    state.system.guild_repo = guild_repo;
    state
}

/// Construit un AppState avec un mock daily activity repository injecte.
pub fn build_test_state_daily_activity(
    daily_activity_repo: Arc<dyn DailyActivityRepository>,
) -> AppState {
    let mut state = base_state();
    state.community.daily_activity_repo = daily_activity_repo;
    state
}

/// Construit un AppState avec un mock analytics repository injecte.
pub fn build_test_state_analytics(analytics_repo: Arc<dyn AnalyticsRepository>) -> AppState {
    let mut state = base_state();
    state.audit.analytics_repo = analytics_repo;
    state
}

/// Construit un AppState avec un mock role panels use case injecte.


/// Construit un AppState avec un mock welcome config repository injecte.
/// Le repo est wrappe dans le service applicatif pour exposer le use case
/// (l'AppState n'expose plus le repo directement).
pub fn build_test_state_welcome(welcome_config_repo: Arc<dyn WelcomeConfigRepository>) -> AppState {
    let mut state = base_state();
    state.community.welcome_config_uc = Arc::new(
        sentinel_core::application::community::manage_welcome_config_service::ManageWelcomeConfigService::new(
            welcome_config_repo,
        ),
    );
    state
}

/// Construit un AppState avec un mock bot_config repository injecte.
pub fn build_test_state_bot_config(bot_config_repo: Arc<dyn BotConfigRepository>) -> AppState {
    let mut state = base_state();
    state.bot_config_repo = bot_config_repo;
    state
}

/// Construit un AppState avec un mock DiscordApi injecte.
pub fn build_test_state_discord_api(discord_api: Arc<dyn DiscordApi>) -> AppState {
    let mut state = base_state();
    state.discord_api = discord_api;
    state
}

// ══════════════════════════════════════════════════════════
// Mock DiscordApi — retourne Ok(()) par defaut pour tous les appels.
// Utilise par les tests qui veulent couvrir le code APRES discord_api
// (log_action + broadcast dans execute_ban/mute/unban, etc.).
// ══════════════════════════════════════════════════════════

#[derive(Default)]
pub struct MockDiscordApi {
    pub calls: std::sync::Mutex<Vec<String>>,
}

impl MockDiscordApi {
    pub fn new() -> Self {
        Self::default()
    }
    fn record(&self, call: &str) {
        self.calls.lock().unwrap().push(call.into());
    }
}

#[async_trait]
impl DiscordApi for MockDiscordApi {
    async fn list_text_channels(&self, _: &str) -> Result<Vec<DiscordChannel>, DomainError> {
        self.record("list_text_channels");
        Ok(vec![])
    }
    async fn list_all_channels(&self, _: &str) -> Result<Vec<DiscordChannel>, DomainError> {
        self.record("list_all_channels");
        Ok(vec![])
    }
    async fn upload_emoji(
        &self,
        _: &str,
        _: &str,
        _: &[u8],
        _: &str,
    ) -> Result<(String, String, bool), DomainError> {
        self.record("upload_emoji");
        Ok(("emoji_id".into(), "emoji_name".into(), false))
    }
    async fn ban_user(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        self.record("ban_user");
        Ok(())
    }
    async fn list_members(&self, _: &str, _: u32) -> Result<Vec<DiscordMember>, DomainError> {
        self.record("list_members");
        Ok(vec![])
    }
    async fn send_dm(&self, _: &str, _: &str) -> Result<(), DomainError> {
        self.record("send_dm");
        Ok(())
    }
    async fn create_role(
        &self,
        _: &str,
        _: &str,
        _: u32,
        _: Option<&str>,
    ) -> Result<serde_json::Value, DomainError> {
        self.record("create_role");
        Ok(serde_json::json!({"id": "r1", "name": "role"}))
    }
    async fn edit_role(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<u32>,
        _: Option<&str>,
        _: Option<bool>,
        _: Option<bool>,
    ) -> Result<serde_json::Value, DomainError> {
        self.record("edit_role");
        Ok(serde_json::json!({"id": "r1"}))
    }
    async fn delete_role(&self, _: &str, _: &str) -> Result<(), DomainError> {
        self.record("delete_role");
        Ok(())
    }
    async fn unban_user(&self, _: &str, _: &str) -> Result<(), DomainError> {
        self.record("unban_user");
        Ok(())
    }
    async fn remove_timeout(&self, _: &str, _: &str) -> Result<(), DomainError> {
        self.record("remove_timeout");
        Ok(())
    }
    async fn apply_timeout(&self, _: &str, _: &str, _: u64) -> Result<(), DomainError> {
        self.record("apply_timeout");
        Ok(())
    }
    async fn get_user_guilds(&self, _: &str) -> Result<Vec<UserGuild>, DomainError> {
        self.record("get_user_guilds");
        Ok(vec![])
    }
    async fn get_user_me(&self, _: &str) -> Result<DiscordUser, DomainError> {
        self.record("get_user_me");
        Ok(DiscordUser {
            id: "u1".into(),
            username: "mock".into(),
            avatar: None,
            global_name: None,
        })
    }
}

// ── Stub Voice Channels (needed for base_state) ──


