//! Tests ManageWatchedUsersService : pass-throughs + 404 dossier.
//! get_user_dossier est couvert par integration HTTP watched_users_http.

use async_trait::async_trait;
use std::sync::Arc;
use std::sync::Mutex;

use crate::application::audit::manage_watched_users_service::ManageWatchedUsersService;
use crate::domain::entities::audit::dashboard_stats::DashboardStats;
use crate::domain::entities::audit::security_event::SecurityEvent;
use crate::domain::entities::audit::user_stats::GuildStatsOverview;
use crate::domain::entities::audit::user_stats::GuildVoiceStats;
use crate::domain::entities::audit::user_stats::UserStats;
use crate::domain::entities::audit::watched_user::WatchedUser;
use crate::domain::entities::moderation::action::applied::ModerationAction;
use crate::domain::entities::moderation::action::applied::UserModerationHistory;
use crate::domain::entities::moderation::infraction::Infraction;
use crate::domain::errors::DomainError;
use crate::ports::inbound::audit::manage_security::AnalyzeNewMemberCommand;
use crate::ports::inbound::audit::manage_security::ManageSecurityUseCase;
use crate::ports::inbound::audit::manage_security::ReportSecurityEventCommand;
use crate::ports::inbound::audit::manage_security::SecurityDecision;
use crate::ports::inbound::audit::manage_stats::RecordMessagesCommand;
use crate::ports::inbound::audit::manage_stats::RecordVoiceCommand;
use crate::ports::inbound::audit::manage_watched_users::ManageWatchedUsersUseCase;
use crate::ports::inbound::moderation::manage_infractions::InfractionFilters;
use crate::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use crate::ports::inbound::moderation::manage_moderation::LogModerationCommand;
use crate::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use crate::ports::outbound::audit::watched_user_repository::WatchedUserRepository;

fn sample_watched(uid: &str) -> WatchedUser {
    WatchedUser {
        user_id: uid.into(),
        username: uid.into(),
        guild_id: "g".into(),
        guild_name: "Guild".into(),
        risk_level: "low".into(),
        total_warns: 0,
        total_mutes: 0,
        total_bans: 0,
        last_incident_at: None,
        security_events_count: 0,
        first_seen_at: chrono::Utc::now(),
    }
}

#[derive(Default)]
struct MockRepo {
    users: Mutex<Vec<WatchedUser>>,
    adds: Mutex<Vec<(String, String, String, String, String)>>,
    removes: Mutex<Vec<(String, String)>>,
}
#[async_trait]
impl WatchedUserRepository for MockRepo {
    async fn find_watched_users(
        &self,
        _: Option<&str>,
        _: i64,
        _: i64,
    ) -> Result<Vec<WatchedUser>, DomainError> {
        Ok(self.users.lock().unwrap().clone())
    }
    async fn add_manual_watch(
        &self,
        g: &str,
        u: &str,
        n: &str,
        r: &str,
        a: &str,
    ) -> Result<(), DomainError> {
        self.adds
            .lock()
            .unwrap()
            .push((g.into(), u.into(), n.into(), r.into(), a.into()));
        Ok(())
    }
    async fn remove_manual_watch(&self, g: &str, u: &str) -> Result<(), DomainError> {
        self.removes.lock().unwrap().push((g.into(), u.into()));
        Ok(())
    }
}

// ── Stubs UC ──

struct StubInf;
#[async_trait]
impl ManageInfractionsUseCase for StubInf {
    async fn count_user_infractions(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        crate::ports::inbound::moderation::manage_infractions::UserInfractionCounts,
        crate::domain::errors::DomainError,
    > {
        Ok(Default::default())
    }
    async fn list_infractions(
        &self,
        _: &str,
        _: InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError> {
        Ok(vec![])
    }
    async fn list_all_infractions(&self, _: i64, _: i64) -> Result<Vec<Infraction>, DomainError> {
        Ok(vec![])
    }
    async fn count_today(&self) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<Infraction>, DomainError> {
        Ok(None)
    }
    async fn delete_infraction(&self, _: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, DomainError> {
        Ok(0)
    }
}

struct StubMod;
#[async_trait]
impl ManageModerationUseCase for StubMod {
    async fn log_action(&self, _: LogModerationCommand) -> Result<ModerationAction, DomainError> {
        unimplemented!()
    }
    async fn get_history(&self, _: &str, t: &str) -> Result<UserModerationHistory, DomainError> {
        Ok(UserModerationHistory {
            target_id: t.into(),
            target_name: "t".into(),
            total_warns: 0,
            total_mutes: 0,
            total_bans: 0,
            actions: vec![],
        })
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
    async fn delete_action(&self, _: uuid::Uuid) -> Result<bool, DomainError> {
        Ok(false)
    }
}

struct StubSec;
#[async_trait]
impl ManageSecurityUseCase for StubSec {
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
        Ok(vec![])
    }
    async fn analyze_new_member(
        &self,
        _: AnalyzeNewMemberCommand,
    ) -> Result<SecurityDecision, DomainError> {
        unimplemented!()
    }
}

// Notes et Stats ne sont plus des dependances de ManageWatchedUsersService :
// leurs stubs ont ete retires avec le 5e argument du constructeur.

fn make_service(repo: Arc<MockRepo>) -> ManageWatchedUsersService {
    ManageWatchedUsersService::new(
        repo,
        Arc::new(StubInf),
        Arc::new(StubMod),
        Arc::new(StubSec),
    )
}

// Silence unused imports warnings pour le Stats UC non utilise.
fn _silence_unused(
    _: UserStats,
    _: DashboardStats,
    _: GuildStatsOverview,
    _: GuildVoiceStats,
    _: RecordMessagesCommand,
    _: RecordVoiceCommand,
) {
}

#[tokio::test]
async fn list_watched_users_returns_repo_data() {
    let r = Arc::new(MockRepo::default());
    r.users.lock().unwrap().push(sample_watched("u1"));
    r.users.lock().unwrap().push(sample_watched("u2"));
    let svc = make_service(r);
    assert_eq!(
        svc.list_watched_users(Some("g"), 50, 0)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn list_watched_users_none_guild_forwards() {
    let svc = make_service(Arc::new(MockRepo::default()));
    assert!(svc
        .list_watched_users(None, 10, 0)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn add_manual_watch_forwards_with_desktop_source() {
    let r = Arc::new(MockRepo::default());
    let svc = make_service(r.clone());
    svc.add_manual_watch("g1", "u1", "Alice", "Raid suspicion")
        .await
        .unwrap();
    let adds = r.adds.lock().unwrap();
    assert_eq!(adds[0].0, "g1");
    assert_eq!(adds[0].1, "u1");
    assert_eq!(adds[0].2, "Alice");
    assert_eq!(adds[0].3, "Raid suspicion");
    // Source hardcode "desktop" (origine UI admin).
    assert_eq!(adds[0].4, "desktop");
}

#[tokio::test]
async fn remove_manual_watch_forwards() {
    let r = Arc::new(MockRepo::default());
    let svc = make_service(r.clone());
    svc.remove_manual_watch("g1", "u1").await.unwrap();
    assert_eq!(r.removes.lock().unwrap()[0], ("g1".into(), "u1".into()));
}

#[tokio::test]
async fn get_user_dossier_not_found_returns_404() {
    // Repo renvoie vec vide -> .find(...).ok_or_else -> NotFound.
    let svc = make_service(Arc::new(MockRepo::default()));
    let err = svc.get_user_dossier("g", "u1").await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

// ── dossier avec donnees ──

struct RichSec;
#[async_trait]
impl ManageSecurityUseCase for RichSec {
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
        Ok(vec![
            SecurityEvent {
                id: uuid::Uuid::new_v4(),
                guild_id: "g".into(),
                event_type: "raid".into(),
                severity: "high".into(),
                user_ids: vec!["u1".into()],
                description: "".into(),
                created_at: chrono::Utc::now(),
            },
            SecurityEvent {
                id: uuid::Uuid::new_v4(),
                guild_id: "g".into(),
                event_type: "raid".into(),
                severity: "high".into(),
                user_ids: vec!["other".into()],
                description: "".into(),
                created_at: chrono::Utc::now(),
            },
        ])
    }
    async fn analyze_new_member(
        &self,
        _: AnalyzeNewMemberCommand,
    ) -> Result<SecurityDecision, DomainError> {
        unimplemented!()
    }
}

#[tokio::test]
async fn get_user_dossier_filters_security_events_by_user_id() {
    let r = Arc::new(MockRepo::default());
    r.users.lock().unwrap().push(sample_watched("u1"));
    let svc = ManageWatchedUsersService::new(
        r,
        Arc::new(StubInf),
        Arc::new(StubMod),
        Arc::new(RichSec),
    );
    let d = svc.get_user_dossier("g", "u1").await.unwrap();
    // Seul l'evenement qui contient "u1" dans user_ids doit etre retenu.
    assert_eq!(d.security_events.len(), 1);
    assert!(d.security_events[0].user_ids.contains(&"u1".to_string()));
}

#[tokio::test]
async fn get_user_dossier_found_returns_empty_dossier() {
    // Repo renvoie le user mais tous les stubs retournent vide -> dossier vide.
    let r = Arc::new(MockRepo::default());
    r.users.lock().unwrap().push(sample_watched("u1"));
    let svc = make_service(r);
    let d = svc.get_user_dossier("g", "u1").await.unwrap();
    assert_eq!(d.user.user_id.as_str(), "u1");
    assert!(d.infractions.is_empty());
    assert!(d.moderation_actions.is_empty());
    assert!(d.security_events.is_empty());
    assert!(d.notes.is_empty());
}
