use async_trait::async_trait;
use std::sync::Arc;
use std::sync::Mutex;

use crate::sentinel::application::community::manage_role_panels_service::ManageRolePanelsService;
use crate::sentinel::domain::entities::community::role_panel::AutoRole;
use crate::sentinel::domain::entities::community::role_panel::RolePanel;
use crate::sentinel::domain::entities::community::role_panel::RolePanelDetail;
use crate::sentinel::domain::entities::community::role_panel::RolePanelEntry;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_role_panels::CreateAutoRoleCommand;
use crate::sentinel::ports::inbound::community::manage_role_panels::CreateRolePanelCommand;
use crate::sentinel::ports::inbound::community::manage_role_panels::CreateRolePanelEntryCommand;
use crate::sentinel::ports::inbound::community::manage_role_panels::ManageRolePanelsUseCase;
use crate::sentinel::ports::inbound::community::manage_role_panels::SetMessageIdCommand;
use crate::sentinel::ports::outbound::community::role_panel_repository::RolePanelRepository;

#[derive(Default)]
struct MockRepo {
    saved_panels: Mutex<Vec<RolePanel>>,
    saved_entries: Mutex<Vec<RolePanelEntry>>,
    saved_auto_roles: Mutex<Vec<AutoRole>>,
    message_id_updates: Mutex<Vec<(String, String)>>,
    delete_panels: Mutex<Vec<String>>,
    delete_auto_roles: Mutex<Vec<(String, String)>>,
    find_panel_returns: Mutex<Option<RolePanelDetail>>,
}

#[async_trait]
impl RolePanelRepository for MockRepo {
    async fn save_panel(&self, p: &RolePanel) -> Result<(), DomainError> {
        self.saved_panels.lock().unwrap().push(p.clone());
        Ok(())
    }
    async fn save_entries(&self, e: &[RolePanelEntry]) -> Result<(), DomainError> {
        self.saved_entries.lock().unwrap().extend_from_slice(e);
        Ok(())
    }
    async fn find_panel(&self, _: &str) -> Result<Option<RolePanelDetail>, DomainError> {
        Ok(self.find_panel_returns.lock().unwrap().clone())
    }
    async fn find_panel_by_message(&self, _: &str) -> Result<Option<RolePanelDetail>, DomainError> {
        Ok(self.find_panel_returns.lock().unwrap().clone())
    }
    async fn find_panels_by_guild(&self, _: &str) -> Result<Vec<RolePanel>, DomainError> {
        Ok(self.saved_panels.lock().unwrap().clone())
    }
    async fn update_message_id(&self, p: &str, m: &str) -> Result<(), DomainError> {
        self.message_id_updates
            .lock()
            .unwrap()
            .push((p.into(), m.into()));
        Ok(())
    }
    async fn delete_panel(&self, p: &str) -> Result<(), DomainError> {
        self.delete_panels.lock().unwrap().push(p.into());
        Ok(())
    }
    async fn find_auto_roles(&self, _: &str) -> Result<Vec<AutoRole>, DomainError> {
        Ok(self.saved_auto_roles.lock().unwrap().clone())
    }
    async fn save_auto_role(&self, a: &AutoRole) -> Result<(), DomainError> {
        self.saved_auto_roles.lock().unwrap().push(a.clone());
        Ok(())
    }
    async fn delete_auto_role(&self, g: &str, r: &str) -> Result<(), DomainError> {
        self.delete_auto_roles
            .lock()
            .unwrap()
            .push((g.into(), r.into()));
        Ok(())
    }
}

#[tokio::test]
async fn create_panel_saves_panel_and_entries_with_uuids() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageRolePanelsService::new(r.clone());
    let cmd = CreateRolePanelCommand {
        guild_id: "g".into(),
        channel_id: "c".into(),
        title: "T".into(),
        description: "D".into(),
        mode: "unique".into(),
        max_roles: Some(3),
        entries: vec![
            CreateRolePanelEntryCommand {
                role_id: "r1".into(),
                role_name: "Role1".into(),
                emoji: None,
                label: "L1".into(),
                style: "primary".into(),
                position: 0,
            },
            CreateRolePanelEntryCommand {
                role_id: "r2".into(),
                role_name: "Role2".into(),
                emoji: Some("game".into()),
                label: "".into(),
                style: "secondary".into(),
                position: 1,
            },
        ],
    };
    let detail = svc.create_panel(cmd).await.unwrap();
    assert_eq!(detail.panel.title, "T");
    assert_eq!(detail.panel.max_roles, Some(3));
    assert!(detail.panel.enabled);
    assert_eq!(detail.entries.len(), 2);
    // Panel saved + entries saved
    assert_eq!(r.saved_panels.lock().unwrap().len(), 1);
    assert_eq!(r.saved_entries.lock().unwrap().len(), 2);
    // Entries have the same panel_id
    let panel_id = detail.panel.id;
    assert!(detail.entries.iter().all(|e| e.panel_id == panel_id));
}

#[tokio::test]
async fn get_panel_not_found_returns_404() {
    let svc = ManageRolePanelsService::new(Arc::new(MockRepo::default()));
    let err = svc.get_panel("nope").await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test]
async fn get_panel_by_message_forwards() {
    let svc = ManageRolePanelsService::new(Arc::new(MockRepo::default()));
    let r = svc.get_panel_by_message("msg").await.unwrap();
    assert!(r.is_none());
}

#[tokio::test]
async fn list_panels_returns_saved() {
    let r = Arc::new(MockRepo::default());
    r.saved_panels.lock().unwrap().push(RolePanel {
        id: uuid::Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        message_id: None,
        title: "t".into(),
        description: "d".into(),
        mode: "unique".into(),
        max_roles: None,
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    });
    let svc = ManageRolePanelsService::new(r);
    let panels = svc.list_panels("g").await.unwrap();
    assert_eq!(panels.len(), 1);
}

#[tokio::test]
async fn set_message_id_forwards() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageRolePanelsService::new(r.clone());
    svc.set_message_id(SetMessageIdCommand {
        panel_id: "p1".into(),
        message_id: "m1".into(),
    })
    .await
    .unwrap();
    assert_eq!(
        r.message_id_updates.lock().unwrap()[0],
        ("p1".into(), "m1".into())
    );
}

#[tokio::test]
async fn delete_panel_forwards() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageRolePanelsService::new(r.clone());
    svc.delete_panel("p1").await.unwrap();
    assert_eq!(r.delete_panels.lock().unwrap()[0], "p1");
}

#[tokio::test]
async fn add_auto_role_creates_enabled_with_uuid() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageRolePanelsService::new(r.clone());
    let auto = svc
        .add_auto_role(CreateAutoRoleCommand {
            guild_id: "g".into(),
            role_id: "r".into(),
            role_name: "Welcome".into(),
            delay_secs: 60,
        })
        .await
        .unwrap();
    assert!(auto.enabled);
    assert_eq!(auto.delay_secs, 60);
    assert!(!auto.id.is_nil());
    assert_eq!(r.saved_auto_roles.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn delete_auto_role_forwards_guild_and_role() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageRolePanelsService::new(r.clone());
    svc.delete_auto_role("g", "r1").await.unwrap();
    assert_eq!(
        r.delete_auto_roles.lock().unwrap()[0],
        ("g".into(), "r1".into())
    );
}

#[tokio::test]
async fn list_auto_roles_empty() {
    let svc = ManageRolePanelsService::new(Arc::new(MockRepo::default()));
    assert!(svc.list_auto_roles("g").await.unwrap().is_empty());
}
