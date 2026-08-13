use super::*;

use chrono::TimeZone;
use uuid::Uuid;

fn ts() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
}

fn sample_panel() -> RolePanel {
    RolePanel {
        id: Uuid::nil(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        message_id: Some("m".into()),
        title: "Roles".into(),
        description: "Choisis".into(),
        mode: "buttons".into(),
        max_roles: Some(3),
        enabled: true,
        created_at: ts(),
        updated_at: ts(),
    }
}

#[test]
fn role_panel_to_proto_full_mapping() {
    let p = role_panel_to_proto(sample_panel());
    assert_eq!(p.id, Uuid::nil().to_string());
    assert_eq!(p.guild_id, "g");
    assert_eq!(p.channel_id, "c");
    assert_eq!(p.message_id.as_deref(), Some("m"));
    assert_eq!(p.title, "Roles");
    assert_eq!(p.mode, "buttons");
    assert_eq!(p.max_roles, Some(3));
    assert!(p.enabled);
}

#[test]
fn role_panel_to_proto_optional_fields_none() {
    let mut panel = sample_panel();
    panel.message_id = None;
    panel.max_roles = None;
    panel.enabled = false;
    let p = role_panel_to_proto(panel);
    assert!(p.message_id.is_none());
    assert!(p.max_roles.is_none());
    assert!(!p.enabled);
}

#[test]
fn role_panel_entry_to_proto_full_mapping() {
    let e = RolePanelEntry {
        id: Uuid::nil(),
        panel_id: Uuid::nil(),
        role_id: "r1".into(),
        role_name: "Gamer".into(),
        emoji: Some("🎮".into()),
        label: "Joueur".into(),
        style: "primary".into(),
        position: 2,
    };
    let p = role_panel_entry_to_proto(e);
    assert_eq!(p.role_id, "r1");
    assert_eq!(p.label, "Joueur");
    assert_eq!(p.style, "primary");
    assert_eq!(p.position, 2);
    assert_eq!(p.emoji.as_deref(), Some("🎮"));
}

#[test]
fn role_panel_detail_to_proto_includes_entries() {
    let detail = RolePanelDetail {
        panel: sample_panel(),
        entries: vec![
            RolePanelEntry {
                id: Uuid::nil(),
                panel_id: Uuid::nil(),
                role_id: "a".into(),
                role_name: "A".into(),
                emoji: None,
                label: "A".into(),
                style: "primary".into(),
                position: 0,
            },
            RolePanelEntry {
                id: Uuid::nil(),
                panel_id: Uuid::nil(),
                role_id: "b".into(),
                role_name: "B".into(),
                emoji: None,
                label: "B".into(),
                style: "primary".into(),
                position: 1,
            },
        ],
    };
    let p = role_panel_detail_to_proto(detail);
    assert!(p.panel.is_some());
    assert_eq!(p.entries.len(), 2);
    assert_eq!(p.entries[0].role_id, "a");
    assert_eq!(p.entries[1].position, 1);
}

#[test]
fn auto_role_to_proto_full_mapping() {
    let r = AutoRole {
        id: Uuid::nil(),
        guild_id: "g".into(),
        role_id: "r".into(),
        role_name: "Member".into(),
        delay_secs: 60,
        enabled: true,
    };
    let p = auto_role_to_proto(r);
    assert_eq!(p.role_id, "r");
    assert_eq!(p.role_name, "Member");
    assert_eq!(p.delay_secs, 60);
    assert!(p.enabled);
}

// ── RPC tests avec mocks ──

use async_trait::async_trait;
use platform_core::sentinel::domain::entities::community::role_panel::RolePanelDetail;
use platform_core::sentinel::domain::entities::system::discord_role::DiscordRole;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::community::manage_role_panels::CreateAutoRoleCommand;
use platform_core::sentinel::ports::inbound::community::manage_role_panels::CreateRolePanelCommand;
use platform_core::sentinel::ports::inbound::community::manage_role_panels::ManageRolePanelsUseCase;
use platform_core::sentinel::ports::inbound::community::manage_role_panels::SetMessageIdCommand;
use platform_core::sentinel::ports::outbound::community::discord_role_repository::DiscordRoleRepository;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Default)]
struct MockPanelsUc {
    panels: Mutex<Vec<RolePanel>>,
    panel_detail: Mutex<Option<RolePanelDetail>>,
    detail_by_message: Mutex<Option<RolePanelDetail>>,
    set_msg_calls: Mutex<Vec<SetMessageIdCommand>>,
    auto_roles: Mutex<Vec<AutoRole>>,
    get_panel_not_found: Mutex<bool>,
}

#[async_trait]
impl ManageRolePanelsUseCase for MockPanelsUc {
    async fn create_panel(
        &self,
        _: CreateRolePanelCommand,
    ) -> Result<RolePanelDetail, DomainError> {
        unimplemented!()
    }
    async fn get_panel(&self, _: &str) -> Result<RolePanelDetail, DomainError> {
        if *self.get_panel_not_found.lock().unwrap() {
            return Err(DomainError::NotFound("panel".into()));
        }
        self.panel_detail
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| DomainError::NotFound("panel".into()))
    }
    async fn get_panel_by_message(&self, _: &str) -> Result<Option<RolePanelDetail>, DomainError> {
        Ok(self.detail_by_message.lock().unwrap().clone())
    }
    async fn list_panels(&self, _: &str) -> Result<Vec<RolePanel>, DomainError> {
        Ok(self.panels.lock().unwrap().clone())
    }
    async fn set_message_id(&self, cmd: SetMessageIdCommand) -> Result<(), DomainError> {
        self.set_msg_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn delete_panel(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_auto_roles(&self, _: &str) -> Result<Vec<AutoRole>, DomainError> {
        Ok(self.auto_roles.lock().unwrap().clone())
    }
    async fn add_auto_role(&self, _: CreateAutoRoleCommand) -> Result<AutoRole, DomainError> {
        unimplemented!()
    }
    async fn delete_auto_role(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

#[derive(Default)]
struct MockRoleRepo {
    sync_calls: Mutex<Vec<(String, Vec<DiscordRole>)>>,
}

#[async_trait]
impl DiscordRoleRepository for MockRoleRepo {
    async fn sync_roles(&self, g: &str, roles: Vec<DiscordRole>) -> Result<(), DomainError> {
        self.sync_calls.lock().unwrap().push((g.into(), roles));
        Ok(())
    }
    async fn find_by_guild(&self, _: &str) -> Result<Vec<DiscordRole>, DomainError> {
        Ok(vec![])
    }
    async fn find_by_id(&self, _: &str, _: &str) -> Result<Option<DiscordRole>, DomainError> {
        Ok(None)
    }
}

fn grpc(uc: Arc<MockPanelsUc>, repo: Arc<MockRoleRepo>) -> RolePanelsGrpc {
    RolePanelsGrpc {
        uc,
        discord_role_repo: repo,
    }
}

#[tokio::test]
async fn get_panel_not_found_returns_none_not_error() {
    let uc = Arc::new(MockPanelsUc::default());
    *uc.get_panel_not_found.lock().unwrap() = true;
    let g = grpc(uc, Arc::new(MockRoleRepo::default()));
    let resp = g
        .get_panel(Request::new(proto::GetPanelRequest {
            panel_id: Uuid::new_v4().to_string(),
        }))
        .await
        .unwrap();
    assert!(resp.into_inner().panel.is_none());
}

#[tokio::test]
async fn get_panel_found_returns_detail() {
    let uc = Arc::new(MockPanelsUc::default());
    *uc.panel_detail.lock().unwrap() = Some(RolePanelDetail {
        panel: sample_panel(),
        entries: vec![],
    });
    let g = grpc(uc, Arc::new(MockRoleRepo::default()));
    let resp = g
        .get_panel(Request::new(proto::GetPanelRequest {
            panel_id: Uuid::new_v4().to_string(),
        }))
        .await
        .unwrap();
    assert!(resp.into_inner().panel.is_some());
}

#[tokio::test]
async fn get_panel_by_message_some_when_found() {
    let uc = Arc::new(MockPanelsUc::default());
    *uc.detail_by_message.lock().unwrap() = Some(RolePanelDetail {
        panel: sample_panel(),
        entries: vec![],
    });
    let g = grpc(uc, Arc::new(MockRoleRepo::default()));
    let resp = g
        .get_panel_by_message(Request::new(proto::GetPanelByMessageRequest {
            message_id: "m".into(),
        }))
        .await
        .unwrap();
    assert!(resp.into_inner().panel.is_some());
}

#[tokio::test]
async fn get_panel_by_message_none_when_absent() {
    let uc = Arc::new(MockPanelsUc::default());
    let g = grpc(uc, Arc::new(MockRoleRepo::default()));
    let resp = g
        .get_panel_by_message(Request::new(proto::GetPanelByMessageRequest {
            message_id: "m".into(),
        }))
        .await
        .unwrap();
    assert!(resp.into_inner().panel.is_none());
}

#[tokio::test]
async fn list_panels_returns_all() {
    let uc = Arc::new(MockPanelsUc::default());
    uc.panels
        .lock()
        .unwrap()
        .extend(vec![sample_panel(), sample_panel()]);
    let g = grpc(uc, Arc::new(MockRoleRepo::default()));
    let resp = g
        .list_panels(Request::new(proto::ListPanelsRequest {
            guild_id: "g".into(),
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().panels.len(), 2);
}

#[tokio::test]
async fn set_message_id_delegates_command_to_uc() {
    let uc = Arc::new(MockPanelsUc::default());
    let g = grpc(uc.clone(), Arc::new(MockRoleRepo::default()));
    let _ = g
        .set_message_id(Request::new(proto::SetMessageIdRequest {
            panel_id: "p1".into(),
            message_id: "m1".into(),
        }))
        .await
        .unwrap();
    let calls = uc.set_msg_calls.lock().unwrap();
    assert_eq!(calls[0].panel_id, "p1");
    assert_eq!(calls[0].message_id, "m1".into());
}

#[tokio::test]
async fn list_auto_roles_returns_list() {
    let uc = Arc::new(MockPanelsUc::default());
    uc.auto_roles.lock().unwrap().push(AutoRole {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        role_id: "r".into(),
        role_name: "N".into(),
        delay_secs: 30,
        enabled: true,
    });
    let g = grpc(uc, Arc::new(MockRoleRepo::default()));
    let resp = g
        .list_auto_roles(Request::new(proto::ListAutoRolesRequest {
            guild_id: "g".into(),
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().roles.len(), 1);
}

#[tokio::test]
async fn sync_discord_roles_returns_synced_count() {
    let repo = Arc::new(MockRoleRepo::default());
    let g = grpc(Arc::new(MockPanelsUc::default()), repo.clone());
    let resp = g
        .sync_discord_roles(Request::new(proto::SyncDiscordRolesRequest {
            guild_id: "g".into(),
            roles: vec![
                proto::SyncDiscordRole {
                    id: "r1".into(),
                    name: "A".into(),
                    color: 0,
                    position: 1,
                    permissions: "8".into(),
                    mentionable: false,
                    managed: false,
                    icon: None,
                    member_count: 10,
                },
                proto::SyncDiscordRole {
                    id: "r2".into(),
                    name: "B".into(),
                    color: 0,
                    position: 2,
                    permissions: "invalid".into(),
                    mentionable: true,
                    managed: true,
                    icon: Some("hash".into()),
                    member_count: 0,
                },
            ],
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().synced, 2);

    let calls = repo.sync_calls.lock().unwrap();
    assert_eq!(calls[0].0, "g");
    assert_eq!(calls[0].1.len(), 2);
    // Permissions "invalid" → fallback 0
    assert_eq!(calls[0].1[1].permissions, 0);
    assert_eq!(calls[0].1[0].permissions, 8);
}

#[tokio::test]
async fn sync_discord_roles_empty_list() {
    let repo = Arc::new(MockRoleRepo::default());
    let g = grpc(Arc::new(MockPanelsUc::default()), repo.clone());
    let resp = g
        .sync_discord_roles(Request::new(proto::SyncDiscordRolesRequest {
            guild_id: "g".into(),
            roles: vec![],
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().synced, 0);
}
