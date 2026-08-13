use async_trait::async_trait;

use crate::sentinel::domain::entities::community::role_panel::AutoRole;
use crate::sentinel::domain::entities::community::role_panel::RolePanel;
use crate::sentinel::domain::entities::community::role_panel::RolePanelDetail;
use crate::sentinel::domain::entities::community::role_panel::RolePanelEntry;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait RolePanelRepository: Send + Sync {
    async fn save_panel(&self, panel: &RolePanel) -> Result<(), DomainError>;
    async fn save_entries(&self, entries: &[RolePanelEntry]) -> Result<(), DomainError>;
    async fn find_panel(&self, panel_id: &str) -> Result<Option<RolePanelDetail>, DomainError>;
    async fn find_panel_by_message(
        &self,
        message_id: &str,
    ) -> Result<Option<RolePanelDetail>, DomainError>;
    async fn find_panels_by_guild(&self, guild_id: &str) -> Result<Vec<RolePanel>, DomainError>;
    async fn update_message_id(&self, panel_id: &str, message_id: &str) -> Result<(), DomainError>;
    async fn delete_panel(&self, panel_id: &str) -> Result<(), DomainError>;
    async fn find_auto_roles(&self, guild_id: &str) -> Result<Vec<AutoRole>, DomainError>;
    async fn save_auto_role(&self, auto_role: &AutoRole) -> Result<(), DomainError>;
    async fn delete_auto_role(&self, guild_id: &str, role_id: &str) -> Result<(), DomainError>;
}
