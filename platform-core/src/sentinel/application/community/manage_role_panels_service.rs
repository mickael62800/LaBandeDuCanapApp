use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::role_panel::AutoRole;
use crate::sentinel::domain::entities::community::role_panel::RolePanel;
use crate::sentinel::domain::entities::community::role_panel::RolePanelDetail;
use crate::sentinel::domain::entities::community::role_panel::RolePanelEntry;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_role_panels::CreateAutoRoleCommand;
use crate::sentinel::ports::inbound::community::manage_role_panels::CreateRolePanelCommand;
use crate::sentinel::ports::inbound::community::manage_role_panels::ManageRolePanelsUseCase;
use crate::sentinel::ports::inbound::community::manage_role_panels::SetMessageIdCommand;
use crate::sentinel::ports::outbound::community::role_panel_repository::RolePanelRepository;

pub struct ManageRolePanelsService {
    repo: Arc<dyn RolePanelRepository>,
}

impl ManageRolePanelsService {
    pub fn new(repo: Arc<dyn RolePanelRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageRolePanelsUseCase for ManageRolePanelsService {
    async fn create_panel(
        &self,
        cmd: CreateRolePanelCommand,
    ) -> Result<RolePanelDetail, DomainError> {
        let now = Utc::now();
        let panel_id = Uuid::new_v4();

        let panel = RolePanel {
            id: panel_id,
            guild_id: cmd.guild_id,
            channel_id: cmd.channel_id,
            message_id: None,
            title: cmd.title,
            description: cmd.description,
            mode: cmd.mode,
            max_roles: cmd.max_roles,
            enabled: true,
            created_at: now,
            updated_at: now,
        };

        let entries: Vec<RolePanelEntry> = cmd
            .entries
            .into_iter()
            .map(|e| RolePanelEntry {
                id: Uuid::new_v4(),
                panel_id,
                role_id: e.role_id,
                role_name: e.role_name,
                emoji: e.emoji,
                label: e.label,
                style: e.style,
                position: e.position,
            })
            .collect();

        self.repo.save_panel(&panel).await?;
        self.repo.save_entries(&entries).await?;

        Ok(RolePanelDetail { panel, entries })
    }

    async fn get_panel(&self, panel_id: &str) -> Result<RolePanelDetail, DomainError> {
        self.repo
            .find_panel(panel_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Panel introuvable : {panel_id}")))
    }

    async fn get_panel_by_message(
        &self,
        message_id: &str,
    ) -> Result<Option<RolePanelDetail>, DomainError> {
        self.repo.find_panel_by_message(message_id).await
    }

    async fn list_panels(&self, guild_id: &str) -> Result<Vec<RolePanel>, DomainError> {
        self.repo.find_panels_by_guild(guild_id).await
    }

    async fn set_message_id(&self, cmd: SetMessageIdCommand) -> Result<(), DomainError> {
        self.repo
            .update_message_id(&cmd.panel_id, &cmd.message_id)
            .await
    }

    async fn delete_panel(&self, panel_id: &str) -> Result<(), DomainError> {
        self.repo.delete_panel(panel_id).await
    }

    async fn list_auto_roles(&self, guild_id: &str) -> Result<Vec<AutoRole>, DomainError> {
        self.repo.find_auto_roles(guild_id).await
    }

    async fn add_auto_role(&self, cmd: CreateAutoRoleCommand) -> Result<AutoRole, DomainError> {
        let auto_role = AutoRole {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            role_id: cmd.role_id,
            role_name: cmd.role_name,
            delay_secs: cmd.delay_secs,
            enabled: true,
        };
        self.repo.save_auto_role(&auto_role).await?;
        Ok(auto_role)
    }

    async fn delete_auto_role(&self, guild_id: &str, role_id: &str) -> Result<(), DomainError> {
        self.repo.delete_auto_role(guild_id, role_id).await
    }
}

