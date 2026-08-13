use super::*;

use chrono::TimeZone;
use platform_core::sentinel::domain::enums::moderation::moderation_gravity::ModerationGravity;
use uuid::Uuid;

fn ts() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
}

fn sample_action() -> ModerationAction {
    ModerationAction {
        id: Uuid::nil(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "mod".into(),
        moderator_name: "Mod".into(),
        target_id: "u".into(),
        target_name: "Joe".into(),
        target_display_name: None,
        action_type: "warn".into(),
        reason: "spam".into(),
        gravity: Some(ModerationGravity::High),
        duration: Some(3600),
        created_at: ts(),
    }
}

#[test]
fn moderation_action_to_proto_full_mapping() {
    let p = moderation_action_to_proto(sample_action());
    assert_eq!(p.guild_id, "g");
    assert_eq!(p.moderator_name, "Mod");
    assert_eq!(p.action_type, "warn");
    assert_eq!(p.reason, "spam");
    assert_eq!(p.gravity.as_deref(), Some("high"));
    assert_eq!(p.duration, Some(3600));
    assert_eq!(p.created_at, ts().to_rfc3339());
}

#[test]
fn moderation_action_to_proto_no_gravity_no_duration() {
    let mut a = sample_action();
    a.gravity = None;
    a.duration = None;
    let p = moderation_action_to_proto(a);
    assert!(p.gravity.is_none());
    assert!(p.duration.is_none());
}

#[test]
fn moderation_action_gravity_low_serialised() {
    let mut a = sample_action();
    a.gravity = Some(ModerationGravity::Low);
    let p = moderation_action_to_proto(a);
    assert_eq!(p.gravity.as_deref(), Some("low"));
}

#[test]
fn user_history_to_proto_full_mapping() {
    let h = UserModerationHistory {
        target_id: "u".into(),
        target_name: "Joe".into(),
        total_warns: 3,
        total_mutes: 1,
        total_bans: 0,
        actions: vec![sample_action(), sample_action()],
    };
    let p = user_history_to_proto(h);
    assert_eq!(p.target_id, "u");
    assert_eq!(p.total_warns, 3);
    assert_eq!(p.total_mutes, 1);
    assert_eq!(p.total_bans, 0);
    assert_eq!(p.actions.len(), 2);
}

#[test]
fn user_history_to_proto_empty_history() {
    let h = UserModerationHistory {
        target_id: "u".into(),
        target_name: "Clean".into(),
        total_warns: 0,
        total_mutes: 0,
        total_bans: 0,
        actions: vec![],
    };
    let p = user_history_to_proto(h);
    assert!(p.actions.is_empty());
}

// ── RPC tests avec mock ──

use async_trait::async_trait;
use chrono::Utc;
use platform_core::sentinel::domain::entities::moderation::action::strikes::StrikeResult;
use platform_core::sentinel::domain::entities::moderation::action::strikes::UserStrike;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::moderation::manage_moderation::LogModerationCommand;
use platform_core::sentinel::ports::inbound::moderation::manage_moderation::LoggedModerationAction;
use platform_core::sentinel::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use std::sync::Arc;
use std::sync::Mutex;
#[derive(Default)]
struct MockModerationUc {
    log_calls: Mutex<Vec<LogModerationCommand>>,
    history_return: Mutex<Option<UserModerationHistory>>,
    strike_result: Mutex<Option<StrikeResult>>,
}

fn sample_strike() -> StrikeResult {
    StrikeResult {
        strike: UserStrike {
            id: Uuid::new_v4(),
            guild_id: "g".into(),
            user_id: "u".into(),
            reason: "warn".into(),
            source: "moderation".into(),
            infraction_id: None,
            expires_at: None,
            created_at: Utc::now(),
        },
        active_count: 3,
        escalation_action: Some("mute".into()),
        escalation_duration: Some(1800),
    }
}

#[async_trait]
impl ManageModerationUseCase for MockModerationUc {
    async fn log_action(&self, cmd: LogModerationCommand) -> Result<ModerationAction, DomainError> {
        let action = ModerationAction {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            channel_id: cmd.channel_id.clone(),
            moderator_id: cmd.moderator_id.clone(),
            moderator_name: cmd.moderator_name.clone(),
            target_id: cmd.target_id.clone(),
            target_name: cmd.target_name.clone(),
            target_display_name: None,
            action_type: cmd.action_type.clone(),
            reason: cmd.reason.clone(),
            gravity: None,
            duration: cmd.duration,
            created_at: ts(),
        };
        self.log_calls.lock().unwrap().push(cmd);
        Ok(action)
    }
    async fn log_action_with_strike(
        &self,
        cmd: LogModerationCommand,
    ) -> Result<LoggedModerationAction, DomainError> {
        let action = self.log_action(cmd).await?;
        Ok(LoggedModerationAction {
            action,
            strike: self.strike_result.lock().unwrap().clone(),
        })
    }
    async fn get_history(
        &self,
        _: &str,
        target_id: &str,
    ) -> Result<UserModerationHistory, DomainError> {
        Ok(self
            .history_return
            .lock()
            .unwrap()
            .clone()
            .unwrap_or(UserModerationHistory {
                target_id: target_id.into(),
                target_name: "unknown".into(),
                total_warns: 0,
                total_mutes: 0,
                total_bans: 0,
                actions: vec![],
            }))
    }
    async fn list_bans(
        &self,
        _: Option<&str>,
        _: i64,
        _: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        Ok(vec![])
    }
    async fn list_actions(
        &self,
        _: Option<&str>,
        _: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        Ok(vec![])
    }
    async fn delete_bans_for_user(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_action(&self, _: Uuid) -> Result<bool, DomainError> {
        Ok(true)
    }
}

/// Doubles des ports du dossier de moderation (preuves, relecture, notes,
/// statistiques, apprenti).
///
/// Les tests de ce fichier couvrent `log_action`, `get_history` et
/// `get_member_context` — les seuls chemins ou une conversion proto non
/// triviale peut se tromper. Les autres RPC ne font que deleguer au port sans
/// transformation, d'ou ces doubles qui echouent bruyamment si un test venait
/// a les emprunter sans etre ecrit pour.
macro_rules! stub_non_exerce {
    ($nom:ident) => {
        #[derive(Default)]
        struct $nom;
    };
}

stub_non_exerce!(MockCancelUc);
stub_non_exerce!(MockAssessRiskUc);
stub_non_exerce!(MockModstatsUc);
stub_non_exerce!(MockEvidenceRepo);
stub_non_exerce!(MockReviewRepo);
stub_non_exerce!(MockPendingActionRepo);
stub_non_exerce!(MockInfractionsUc);

#[async_trait::async_trait]
impl
    platform_core::sentinel::ports::inbound::moderation::assess_target_risk::AssessTargetRiskUseCase
    for MockAssessRiskUc
{
    async fn assess(
        &self,
        _cmd: platform_core::sentinel::ports::inbound::moderation::assess_target_risk::AssessTargetRiskCommand,
    ) -> Result<
        platform_core::sentinel::domain::entities::moderation::target_risk::TargetRiskDecision,
        platform_core::sentinel::domain::errors::DomainError,
    > {
        unimplemented!("assess_target_risk non exerce par ces tests")
    }
}

#[async_trait::async_trait]
impl platform_core::sentinel::ports::inbound::moderation::read_modstats::ReadModstatsUseCase
    for MockModstatsUc
{
    async fn modstats(
        &self,
        _guild_id: &str,
        _days: i32,
    ) -> Result<
        Vec<platform_core::sentinel::domain::entities::moderation::modstats::ModeratorBreakdown>,
        platform_core::sentinel::domain::errors::DomainError,
    > {
        unimplemented!("modstats non exerce par ces tests")
    }
    async fn modstats_trend(
        &self,
        _guild_id: &str,
        _days: i32,
    ) -> Result<
        Vec<platform_core::sentinel::domain::entities::moderation::modstats::ModstatsTrendDay>,
        platform_core::sentinel::domain::errors::DomainError,
    > {
        unimplemented!("modstats_trend non exerce par ces tests")
    }
}

#[async_trait::async_trait]
impl platform_core::sentinel::ports::outbound::moderation::evidence_repository::EvidenceRepository
    for MockEvidenceRepo
{
    async fn add(
        &self,
        _action_id: uuid::Uuid,
        _url: &str,
        _description: Option<&str>,
        _uploaded_by: &str,
        _uploaded_by_name: &str,
    ) -> Result<
        platform_core::sentinel::ports::outbound::moderation::evidence_repository::EvidenceEntry,
        platform_core::sentinel::domain::errors::DomainError,
    > {
        unimplemented!("evidence add non exerce par ces tests")
    }
    async fn list(
        &self,
        _action_id: uuid::Uuid,
    ) -> Result<
        Vec<platform_core::sentinel::ports::outbound::moderation::evidence_repository::EvidenceEntry>,
        platform_core::sentinel::domain::errors::DomainError,
    >{
        unimplemented!("evidence list non exerce par ces tests")
    }
}

#[async_trait::async_trait]
impl platform_core::sentinel::ports::outbound::moderation::review_repository::ReviewRepository
    for MockReviewRepo
{
    async fn add(
        &self,
        _action_id: uuid::Uuid,
        _guild_id: &str,
        _added_by: &str,
        _added_by_name: &str,
        _reason: Option<&str>,
    ) -> Result<
        platform_core::sentinel::ports::outbound::moderation::review_repository::ReviewEntry,
        platform_core::sentinel::domain::errors::DomainError,
    > {
        unimplemented!("review add non exerce par ces tests")
    }
    async fn list_pending(
        &self,
        _guild_id: &str,
    ) -> Result<
        Vec<platform_core::sentinel::ports::outbound::moderation::review_repository::ReviewEntry>,
        platform_core::sentinel::domain::errors::DomainError,
    > {
        unimplemented!("review list_pending non exerce par ces tests")
    }
    async fn resolve(
        &self,
        _review_id: uuid::Uuid,
        _reviewer_id: &str,
        _reviewer_name: &str,
        _notes: Option<&str>,
        _status: &str,
    ) -> Result<bool, platform_core::sentinel::domain::errors::DomainError> {
        unimplemented!("review resolve non exerce par ces tests")
    }
    async fn get_guild_id(
        &self,
        _review_id: uuid::Uuid,
    ) -> Result<Option<String>, platform_core::sentinel::domain::errors::DomainError> {
        unimplemented!("review get_guild_id non exerce par ces tests")
    }
}

#[async_trait::async_trait]
impl platform_core::sentinel::ports::outbound::moderation::pending_action_repository::PendingActionRepository
    for MockPendingActionRepo
{
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        _guild_id: &str,
        _moderator_id: &str,
        _moderator_name: &str,
        _target_id: &str,
        _target_name: &str,
        _action_type: &str,
        _reason: &str,
        _gravity: Option<&str>,
        _duration: Option<i64>,
    ) -> Result<uuid::Uuid, platform_core::sentinel::domain::errors::DomainError> {
        unimplemented!("pending create non exerce par ces tests")
    }
    async fn list_pending(
        &self,
        _guild_id: &str,
    ) -> Result<
        Vec<platform_core::sentinel::ports::outbound::moderation::pending_action_repository::PendingAction>,
        platform_core::sentinel::domain::errors::DomainError,
    > {
        unimplemented!("pending list non exerce par ces tests")
    }
    async fn get_guild_id(
        &self,
        _id: uuid::Uuid,
    ) -> Result<Option<String>, platform_core::sentinel::domain::errors::DomainError> {
        unimplemented!("pending get_guild_id non exerce par ces tests")
    }
    async fn resolve(
        &self,
        _id: uuid::Uuid,
        _status: &str,
        _reviewed_by: &str,
    ) -> Result<(), platform_core::sentinel::domain::errors::DomainError> {
        unimplemented!("pending resolve non exerce par ces tests")
    }
}

#[async_trait::async_trait]
impl platform_core::sentinel::ports::inbound::moderation::cancel_action::CancelModerationActionUseCase
    for MockCancelUc
{
    async fn cancel(
        &self,
        _action_id: uuid::Uuid,
    ) -> Result<
        platform_core::sentinel::ports::inbound::moderation::cancel_action::CancelOutcome,
        platform_core::sentinel::domain::errors::DomainError,
    > {
        Ok(platform_core::sentinel::ports::inbound::moderation::cancel_action::CancelOutcome::NotFound)
    }
}

fn grpc(uc: Arc<MockModerationUc>) -> ModerationGrpc {
    ModerationGrpc {
        moderation_uc: uc,
        cancel_action_uc: Arc::new(MockCancelUc),
        assess_target_risk_uc: Arc::new(MockAssessRiskUc),
        modstats_uc: Arc::new(MockModstatsUc),
        evidence_repo: Arc::new(MockEvidenceRepo),
        review_repo: Arc::new(MockReviewRepo),
        pending_action_repo: Arc::new(MockPendingActionRepo),
        infractions_uc: Arc::new(MockInfractionsUc),
        manage_reminders_uc: Arc::new(MockManageRemindersUc),
    }
}

struct MockManageRemindersUc;
#[tonic::async_trait]
impl platform_core::sentinel::ports::inbound::moderation::manage_reminders::ManageRemindersUseCase
    for MockManageRemindersUc
{
    async fn create_reminder(
        &self,
        cmd: platform_core::sentinel::ports::inbound::moderation::manage_reminders::CreateReminderCommand,
    ) -> Result<
        platform_core::sentinel::domain::entities::moderation::action::sanction_reminder::SanctionReminder,
        platform_core::sentinel::domain::errors::DomainError,
    >{
        Ok(platform_core::sentinel::domain::entities::moderation::action::sanction_reminder::SanctionReminder {
            id: uuid::Uuid::new_v4(),
            guild_id: cmd.guild_id,
            moderator_id: cmd.moderator_id,
            moderator_name: cmd.moderator_name,
            target_id: cmd.target_id,
            target_name: cmd.target_name,
            action_type: cmd.action_type,
            reason: cmd.reason,
            action_id: cmd.action_id,
            expires_at: chrono::Utc::now(),
            remind_at: chrono::Utc::now(),
            status: "pending".into(),
            created_at: chrono::Utc::now(),
        })
    }
    async fn get_pending_reminders(&self) -> Result<Vec<platform_core::sentinel::domain::entities::moderation::action::sanction_reminder::SanctionReminder>, platform_core::sentinel::domain::errors::DomainError>{
        unimplemented!()
    }
    async fn mark_sent(
        &self,
        _reminder_id: uuid::Uuid,
    ) -> Result<(), platform_core::sentinel::domain::errors::DomainError> {
        unimplemented!()
    }
    async fn cancel_for_action(
        &self,
        _action_id: uuid::Uuid,
    ) -> Result<(), platform_core::sentinel::domain::errors::DomainError> {
        unimplemented!()
    }
    async fn list_by_guild(&self, _guild_id: &str) -> Result<Vec<platform_core::sentinel::domain::entities::moderation::action::sanction_reminder::SanctionReminder>, platform_core::sentinel::domain::errors::DomainError>{
        unimplemented!()
    }
}

fn make_log_request(action: &str) -> Request<proto::LogActionRequest> {
    Request::new(proto::LogActionRequest {
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "mod".into(),
        moderator_name: "Mod".into(),
        target_id: "t".into(),
        target_name: "Target".into(),
        action_type: action.into(),
        reason: "r".into(),
        gravity: None,
        duration: None,
        skip_strike: false,
    })
}

// BUG #8 : seul ban_temp cree un rappel d'expiration ; mute_temp expire seul via
// le timeout Discord et ne doit PAS generer de rappel.

// BUG #1 : un unban annule les rappels d'auto-unban pour la cible ; un ban ne
// declenche aucune annulation.

#[tokio::test]
async fn log_action_delegates_to_uc() {
    let uc = Arc::new(MockModerationUc::default());
    let g = grpc(uc.clone());
    let _ = g.log_action(make_log_request("warn")).await.unwrap();
    let calls = uc.log_calls.lock().unwrap();
    assert_eq!(calls[0].action_type, "warn");
    assert_eq!(calls[0].moderator_name, "Mod");
}

#[tokio::test]
async fn log_action_without_strike_has_none_escalation() {
    let uc = Arc::new(MockModerationUc::default());
    let g = grpc(uc);
    let resp = g.log_action(make_log_request("warn")).await.unwrap();
    let inner = resp.into_inner();
    assert!(inner.strikes_count.is_none());
    assert!(inner.escalation_action.is_none());
    assert!(inner.escalation_duration.is_none());
}

#[tokio::test]
async fn log_action_with_strike_populates_escalation() {
    let uc = Arc::new(MockModerationUc::default());
    *uc.strike_result.lock().unwrap() = Some(sample_strike());
    let g = grpc(uc);
    let resp = g.log_action(make_log_request("warn")).await.unwrap();
    let inner = resp.into_inner();
    assert_eq!(inner.strikes_count, Some(3));
    assert_eq!(inner.escalation_action.as_deref(), Some("mute"));
    assert_eq!(inner.escalation_duration, Some(1800));
}

#[tokio::test]
async fn get_history_returns_full_user_data() {
    let uc = Arc::new(MockModerationUc::default());
    *uc.history_return.lock().unwrap() = Some(UserModerationHistory {
        target_id: "u".into(),
        target_name: "Alice".into(),
        total_warns: 5,
        total_mutes: 2,
        total_bans: 1,
        actions: vec![sample_action()],
    });
    let g = grpc(uc);
    let resp = g
        .get_history(Request::new(proto::GetHistoryRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
        }))
        .await
        .unwrap();
    let h = resp.into_inner();
    assert_eq!(h.target_name, "Alice");
    assert_eq!(h.total_warns, 5);
    assert_eq!(h.actions.len(), 1);
}

#[tokio::test]
async fn get_history_clean_user_has_zero_counters() {
    let uc = Arc::new(MockModerationUc::default());
    let g = grpc(uc);
    let resp = g
        .get_history(Request::new(proto::GetHistoryRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
        }))
        .await
        .unwrap();
    let h = resp.into_inner();
    assert_eq!(h.total_warns, 0);
    assert_eq!(h.total_mutes, 0);
    assert_eq!(h.total_bans, 0);
    assert!(h.actions.is_empty());
}

#[async_trait::async_trait]
impl platform_core::sentinel::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase
    for MockInfractionsUc
{
    async fn count_user_infractions(
        &self,
        _guild_id: &str,
        _user_id: &str,
    ) -> Result<
        platform_core::sentinel::ports::inbound::moderation::manage_infractions::UserInfractionCounts,
        platform_core::sentinel::domain::errors::DomainError,
    > {
        unimplemented!("count_user_infractions non exerce par ces tests")
    }
    async fn list_infractions(
        &self,
        _guild_id: &str,
        _filters: platform_core::sentinel::ports::inbound::moderation::manage_infractions::InfractionFilters,
    ) -> Result<
        Vec<platform_core::sentinel::domain::entities::moderation::infraction::Infraction>,
        platform_core::sentinel::domain::errors::DomainError,
    > {
        unimplemented!("list_infractions non exerce par ces tests")
    }
    async fn list_all_infractions(
        &self,
        _limit: i64,
        _offset: i64,
    ) -> Result<
        Vec<platform_core::sentinel::domain::entities::moderation::infraction::Infraction>,
        platform_core::sentinel::domain::errors::DomainError,
    > {
        unimplemented!("list_all_infractions non exerce par ces tests")
    }
    async fn count_today(&self) -> Result<u64, platform_core::sentinel::domain::errors::DomainError> {
        unimplemented!("count_today non exerce par ces tests")
    }
    async fn find_by_id(
        &self,
        _id: &str,
    ) -> Result<
        Option<platform_core::sentinel::domain::entities::moderation::infraction::Infraction>,
        platform_core::sentinel::domain::errors::DomainError,
    > {
        unimplemented!("find_by_id non exerce par ces tests")
    }
    async fn delete_infraction(
        &self,
        _id: &str,
    ) -> Result<bool, platform_core::sentinel::domain::errors::DomainError> {
        unimplemented!("delete_infraction non exerce par ces tests")
    }
    async fn delete_older_than_days(
        &self,
        _guild_id: &str,
        _days: i32,
    ) -> Result<u64, platform_core::sentinel::domain::errors::DomainError> {
        unimplemented!("delete_older_than_days non exerce par ces tests")
    }
}
