//! Tests pour ManageMembersService. On couvre les pass-through + get_member
//! (404 path + success). get_member_summary est couvert par les tests HTTP
//! integration (members_http) qui testent le flow complet avec stubs.

use async_trait::async_trait;
use std::sync::Arc;
use std::sync::Mutex;

use crate::sentinel::application::community::manage_members_service::ManageMembersService;
use crate::sentinel::domain::entities::community::guild_member::GuildMember;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_members::ManageMembersUseCase;
use crate::sentinel::ports::inbound::community::manage_members::RegisterMemberCommand;
use crate::sentinel::ports::inbound::community::manage_members::SyncMembersCommand;
use crate::sentinel::ports::inbound::community::manage_members::UpdateMemberCommand;
use crate::sentinel::ports::outbound::community::member_repository::MemberRepository;

fn sample_member(g: &str, u: &str, name: &str) -> GuildMember {
    GuildMember {
        guild_id: g.into(),
        user_id: u.into(),
        username: name.into(),
        display_name: None,
        avatar: None,
        roles: serde_json::json!([]),
        joined_at: None,
        account_created: None,
        is_bot: false,
        last_seen_at: None,
        left_at: None,
    }
}

#[derive(Default)]
struct MockMemberRepo {
    members: Mutex<Vec<GuildMember>>,
    upserts: Mutex<Vec<GuildMember>>,
    upsert_many_calls: Mutex<Vec<Vec<GuildMember>>>,
    deletes: Mutex<Vec<(String, String)>>,
}
#[async_trait]
impl MemberRepository for MockMemberRepo {
    async fn find_by_guild(&self, _: &str) -> Result<Vec<GuildMember>, DomainError> {
        Ok(self.members.lock().unwrap().clone())
    }
    async fn find_one(&self, g: &str, u: &str) -> Result<Option<GuildMember>, DomainError> {
        Ok(self
            .members
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.guild_id.as_str() == g && m.user_id.as_str() == u)
            .cloned())
    }
    async fn upsert(&self, m: &GuildMember) -> Result<(), DomainError> {
        self.upserts.lock().unwrap().push(m.clone());
        Ok(())
    }
    async fn upsert_many(&self, m: &[GuildMember]) -> Result<u64, DomainError> {
        self.upsert_many_calls.lock().unwrap().push(m.to_vec());
        Ok(m.len() as u64)
    }
    async fn delete(&self, g: &str, u: &str) -> Result<(), DomainError> {
        self.deletes.lock().unwrap().push((g.into(), u.into()));
        Ok(())
    }
    async fn update_last_seen(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn is_left(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn reset_member(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Vec<(&'static str, u64)>, DomainError> {
        Ok(vec![])
    }
    async fn mark_left(&self, _: &str, _: &str) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn mark_rejoined(&self, _: &str, _: &str) -> Result<u64, DomainError> {
        Ok(0)
    }

    // Lectures servant la page membre : hors du perimetre teste ici.
    async fn list_join_anniversaries(
        &self,
        _: &str,
        _: i32,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::community::milestone::JoinAnniversary>,
        DomainError,
    > {
        Ok(vec![])
    }

    async fn list_recent_joins(
        &self,
        _: &str,
        _: i32,
        _: i64,
    ) -> Result<Vec<GuildMember>, DomainError> {
        Ok(vec![])
    }
}

// ── Stubs minimaux pour les use cases satellites (non utilises ici) ──

use crate::sentinel::domain::entities::audit::dashboard_stats::DashboardStats;
use crate::sentinel::domain::entities::audit::user_stats::GuildStatsOverview;
use crate::sentinel::domain::entities::audit::user_stats::GuildVoiceStats;
use crate::sentinel::domain::entities::audit::user_stats::UserStats;
use crate::sentinel::domain::entities::moderation::action::applied::ModerationAction;
use crate::sentinel::domain::entities::moderation::action::applied::UserModerationHistory;
use crate::sentinel::domain::entities::moderation::infraction::Infraction;
use crate::sentinel::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use crate::sentinel::ports::inbound::audit::manage_stats::RecordMessagesCommand;
use crate::sentinel::ports::inbound::audit::manage_stats::RecordVoiceCommand;
use crate::sentinel::ports::inbound::moderation::manage_infractions::InfractionFilters;
use crate::sentinel::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use crate::sentinel::ports::inbound::moderation::manage_moderation::LogModerationCommand;
use crate::sentinel::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
struct StubInfUc;
#[async_trait]
impl ManageInfractionsUseCase for StubInfUc {
    async fn count_user_infractions(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        crate::sentinel::ports::inbound::moderation::manage_infractions::UserInfractionCounts,
        crate::sentinel::domain::errors::DomainError,
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

struct StubModUc;
#[async_trait]
impl ManageModerationUseCase for StubModUc {
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

struct StubStatsUc;
#[async_trait]
impl ManageStatsUseCase for StubStatsUc {
    async fn record_messages(&self, _: RecordMessagesCommand) -> Result<(), DomainError> {
        Ok(())
    }
    async fn record_voice(&self, _: RecordVoiceCommand) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_user_stats(&self, _: &str, _: &str) -> Result<Option<UserStats>, DomainError> {
        Ok(None)
    }
    async fn get_guild_overview(&self, _: &str) -> Result<GuildStatsOverview, DomainError> {
        unimplemented!()
    }
    async fn get_leaderboard(&self, _: &str, _: u32) -> Result<Vec<UserStats>, DomainError> {
        Ok(vec![])
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

fn make_service(repo: Arc<MockMemberRepo>) -> ManageMembersService {
    ManageMembersService::new(
        repo,
        Arc::new(StubInfUc),
        Arc::new(StubModUc),
        Arc::new(StubStatsUc),
    )
}

#[tokio::test]
async fn list_members_returns_repo_data() {
    let r = Arc::new(MockMemberRepo::default());
    r.members
        .lock()
        .unwrap()
        .push(sample_member("g", "u", "Alice"));
    let svc = make_service(r);
    let members = svc.list_members("g").await.unwrap();
    assert_eq!(members.len(), 1);
}

#[tokio::test]
async fn get_member_not_found_returns_404() {
    let svc = make_service(Arc::new(MockMemberRepo::default()));
    let err = svc.get_member("g", "u").await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test]
async fn get_member_found_returns_member() {
    let r = Arc::new(MockMemberRepo::default());
    r.members
        .lock()
        .unwrap()
        .push(sample_member("g", "u", "Alice"));
    let svc = make_service(r);
    let m = svc.get_member("g", "u").await.unwrap();
    assert_eq!(m.username, "Alice");
}

#[tokio::test]
async fn sync_members_returns_count() {
    let r = Arc::new(MockMemberRepo::default());
    let svc = make_service(r.clone());
    let n = svc
        .sync_members(SyncMembersCommand {
            guild_id: "g".into(),
            members: vec![sample_member("g", "u1", "A"), sample_member("g", "u2", "B")],
        })
        .await
        .unwrap();
    assert_eq!(n, 2);
    assert_eq!(r.upsert_many_calls.lock().unwrap()[0].len(), 2);
}

#[tokio::test]
async fn register_member_forwards_upsert() {
    let r = Arc::new(MockMemberRepo::default());
    let svc = make_service(r.clone());
    svc.register_member(RegisterMemberCommand {
        member: sample_member("g", "u", "A"),
    })
    .await
    .unwrap();
    assert_eq!(r.upserts.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn remove_member_forwards_delete() {
    let r = Arc::new(MockMemberRepo::default());
    let svc = make_service(r.clone());
    svc.remove_member("g", "u").await.unwrap();
    assert_eq!(r.deletes.lock().unwrap()[0], ("g".into(), "u".into()));
}

#[tokio::test]
async fn update_member_applies_partial_fields() {
    let r = Arc::new(MockMemberRepo::default());
    r.members
        .lock()
        .unwrap()
        .push(sample_member("g", "u", "OldName"));
    let svc = make_service(r.clone());
    svc.update_member(UpdateMemberCommand {
        guild_id: "g".into(),
        user_id: "u".into(),
        username: Some("NewName".into()),
        display_name: Some("Display".into()),
        avatar: None,
        roles: None,
    })
    .await
    .unwrap();
    let upserted = &r.upserts.lock().unwrap()[0];
    assert_eq!(upserted.username, "NewName");
    assert_eq!(upserted.display_name.as_deref(), Some("Display"));
}

#[tokio::test]
async fn update_member_not_found_returns_404() {
    let svc = make_service(Arc::new(MockMemberRepo::default()));
    let err = svc
        .update_member(UpdateMemberCommand {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: Some("X".into()),
            display_name: None,
            avatar: None,
            roles: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test]
async fn update_member_applies_avatar_and_roles() {
    let r = Arc::new(MockMemberRepo::default());
    r.members.lock().unwrap().push(sample_member("g", "u", "X"));
    let svc = make_service(r.clone());
    svc.update_member(UpdateMemberCommand {
        guild_id: "g".into(),
        user_id: "u".into(),
        username: None,
        display_name: None,
        avatar: Some("https://img/av.png".into()),
        roles: Some(serde_json::json!(["mod"])),
    })
    .await
    .unwrap();
    let up = &r.upserts.lock().unwrap()[0];
    assert_eq!(up.avatar.as_deref(), Some("https://img/av.png"));
    assert_eq!(up.roles, serde_json::json!(["mod"]));
    // Unchanged:
    assert_eq!(up.username, "X");
}

// ── get_member_summary ──

#[tokio::test]
async fn get_member_summary_returns_default_shape_for_empty_stubs() {
    let r = Arc::new(MockMemberRepo::default());
    r.members
        .lock()
        .unwrap()
        .push(sample_member("g", "u", "Alice"));
    let svc = make_service(r);
    let s = svc.get_member_summary("g", "u").await.unwrap();
    assert_eq!(s.member.username, "Alice");
    assert_eq!(s.infractions.total, 0);
    assert!(s.infractions.recent.is_empty());
    assert_eq!(s.moderation.total_warns, 0);
    assert_eq!(s.moderation.total_mutes, 0);
    assert_eq!(s.moderation.total_bans, 0);
    // StatsUc renvoie None → 0/0/None.
    assert_eq!(s.stats.message_count, 0);
    assert_eq!(s.stats.voice_seconds, 0);
    assert!(s.stats.last_active.is_none());
}

#[tokio::test]
async fn get_member_summary_not_found_returns_404() {
    let svc = make_service(Arc::new(MockMemberRepo::default()));
    let err = svc.get_member_summary("g", "ghost").await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

// ── get_member_summary avec stubs enrichis (infractions + moderation + stats) ──

struct RichInfUc;
#[async_trait]
impl ManageInfractionsUseCase for RichInfUc {
    async fn count_user_infractions(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        crate::sentinel::ports::inbound::moderation::manage_infractions::UserInfractionCounts,
        crate::sentinel::domain::errors::DomainError,
    > {
        Ok(Default::default())
    }
    async fn list_infractions(
        &self,
        g: &str,
        _: InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError> {
        let now = chrono::Utc::now();
        Ok((0..3)
            .map(|i| Infraction {
                id: uuid::Uuid::new_v4(),
                guild_id: g.into(),
                channel_id: "c".into(),
                user_id: "u".into(),
                username: "u".into(),
                display_name: None,
                message_id: "m".into(),
                content: format!("msg{i}"),
                flags:
                    crate::sentinel::domain::entities::moderation::detection_flags::DetectionFlags {
                        spam: false,
                        insult: false,
                        profanity: false,
                        link: false,
                        phishing: false,
                    },
                action: crate::sentinel::domain::enums::moderation::action::Action::Warn,
                score: 1.0 + i as f64,
                reason: format!("r{i}"),
                duration: None,
                created_at: now,
            })
            .collect())
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

struct RichModUc;
#[async_trait]
impl ManageModerationUseCase for RichModUc {
    async fn log_action(&self, _: LogModerationCommand) -> Result<ModerationAction, DomainError> {
        unimplemented!()
    }
    async fn get_history(&self, g: &str, t: &str) -> Result<UserModerationHistory, DomainError> {
        let now = chrono::Utc::now();
        let mk = |kind: &str| ModerationAction {
            id: uuid::Uuid::new_v4(),
            guild_id: g.into(),
            channel_id: "c".into(),
            target_id: t.into(),
            target_name: "t".into(),
            target_display_name: None,
            moderator_id: "m".into(),
            moderator_name: "M".into(),
            action_type: kind.into(),
            reason: "r".into(),
            gravity: None,
            duration: None,
            created_at: now,
        };
        Ok(UserModerationHistory {
            target_id: t.into(),
            target_name: "t".into(),
            total_warns: 2,
            total_mutes: 1,
            total_bans: 0,
            actions: vec![mk("warn"), mk("warn"), mk("mute")],
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

struct RichStatsUc;
#[async_trait]
impl ManageStatsUseCase for RichStatsUc {
    async fn record_messages(&self, _: RecordMessagesCommand) -> Result<(), DomainError> {
        Ok(())
    }
    async fn record_voice(&self, _: RecordVoiceCommand) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_user_stats(&self, g: &str, u: &str) -> Result<Option<UserStats>, DomainError> {
        let now = chrono::Utc::now();
        Ok(Some(UserStats {
            id: uuid::Uuid::new_v4(),
            guild_id: g.into(),
            user_id: u.into(),
            username: u.into(),
            message_count: 42,
            voice_seconds: 3600,
            updated_at: now,
        }))
    }
    async fn get_guild_overview(&self, _: &str) -> Result<GuildStatsOverview, DomainError> {
        unimplemented!()
    }
    async fn get_leaderboard(&self, _: &str, _: u32) -> Result<Vec<UserStats>, DomainError> {
        Ok(vec![])
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

#[tokio::test]
async fn get_member_summary_counts_moderation_actions_by_type() {
    let r = Arc::new(MockMemberRepo::default());
    r.members
        .lock()
        .unwrap()
        .push(sample_member("g", "u", "Alice"));
    let svc = ManageMembersService::new(
        r,
        Arc::new(RichInfUc),
        Arc::new(RichModUc),
        Arc::new(RichStatsUc),
    );
    let s = svc.get_member_summary("g", "u").await.unwrap();
    assert_eq!(s.moderation.total_warns, 2);
    assert_eq!(s.moderation.total_mutes, 1);
    assert_eq!(s.moderation.total_bans, 0);
    assert_eq!(s.moderation.actions.len(), 3);
    assert_eq!(s.infractions.total, 3);
    assert_eq!(s.infractions.recent.len(), 3);
    assert_eq!(s.stats.message_count, 42);
    assert_eq!(s.stats.voice_seconds, 3600);
    assert!(s.stats.last_active.is_some());
}
