use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::role_panel::AutoRole;
use platform_core::sentinel::domain::entities::community::role_panel::RolePanel;
use platform_core::sentinel::domain::entities::community::role_panel::RolePanelDetail;
use platform_core::sentinel::domain::entities::community::role_panel::RolePanelEntry;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::role_panel_repository::RolePanelRepository;

pub struct PgRolePanelRepository {
    pool: PgPool,
}

impl PgRolePanelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct PanelRow {
    id: Uuid,
    guild_id: String,
    channel_id: String,
    message_id: Option<String>,
    title: String,
    description: String,
    mode: String,
    max_roles: Option<i32>,
    enabled: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct EntryRow {
    id: Uuid,
    panel_id: Uuid,
    role_id: String,
    role_name: String,
    emoji: Option<String>,
    label: String,
    style: String,
    position: i32,
}

#[derive(sqlx::FromRow)]
struct AutoRoleRow {
    id: Uuid,
    guild_id: String,
    role_id: String,
    role_name: String,
    delay_secs: i32,
    enabled: bool,
}

impl From<PanelRow> for RolePanel {
    fn from(r: PanelRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            channel_id: r.channel_id.into(),
            message_id: r.message_id,
            title: r.title,
            description: r.description,
            mode: r.mode,
            max_roles: r.max_roles,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

impl From<EntryRow> for RolePanelEntry {
    fn from(r: EntryRow) -> Self {
        Self {
            id: r.id,
            panel_id: r.panel_id,
            role_id: r.role_id.into(),
            role_name: r.role_name,
            emoji: r.emoji,
            label: r.label,
            style: r.style,
            position: r.position,
        }
    }
}

impl From<AutoRoleRow> for AutoRole {
    fn from(r: AutoRoleRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            role_id: r.role_id.into(),
            role_name: r.role_name,
            delay_secs: r.delay_secs,
            enabled: r.enabled,
        }
    }
}

#[async_trait]
impl RolePanelRepository for PgRolePanelRepository {
    async fn save_panel(&self, panel: &RolePanel) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO role_panels (id, guild_id, channel_id, message_id, title, description, mode, max_roles, enabled, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)"
        )
        .bind(panel.id).bind(panel.guild_id.as_str()).bind(panel.channel_id.as_str()).bind(panel.message_id.as_deref())
        .bind(&panel.title).bind(&panel.description).bind(&panel.mode).bind(panel.max_roles)
        .bind(panel.enabled).bind(panel.created_at).bind(panel.updated_at)
        .execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }

    async fn save_entries(&self, entries: &[RolePanelEntry]) -> Result<(), DomainError> {
        for entry in entries {
            sqlx::query(
                "INSERT INTO role_panel_entries (id, panel_id, role_id, role_name, emoji, label, style, position) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)"
            )
            .bind(entry.id).bind(entry.panel_id).bind(entry.role_id.as_str()).bind(&entry.role_name)
            .bind(&entry.emoji).bind(&entry.label).bind(&entry.style).bind(entry.position)
            .execute(&self.pool).await.map_err(pg_err)?;
        }
        Ok(())
    }

    async fn find_panel(&self, panel_id: &str) -> Result<Option<RolePanelDetail>, DomainError> {
        let id: Uuid = panel_id
            .parse()
            .map_err(|_| DomainError::ValidationError("ID invalide".into()))?;
        let panel = sqlx::query_as::<_, PanelRow>("SELECT * FROM role_panels WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        match panel {
            Some(p) => {
                let entries = sqlx::query_as::<_, EntryRow>(
                    "SELECT * FROM role_panel_entries WHERE panel_id = $1 ORDER BY position ASC",
                )
                .bind(id)
                .fetch_all(&self.pool)
                .await
                .map_err(pg_err)?;
                Ok(Some(RolePanelDetail {
                    panel: p.into(),
                    entries: entries.into_iter().map(RolePanelEntry::from).collect(),
                }))
            }
            None => Ok(None),
        }
    }

    async fn find_panel_by_message(
        &self,
        message_id: &str,
    ) -> Result<Option<RolePanelDetail>, DomainError> {
        let panel =
            sqlx::query_as::<_, PanelRow>("SELECT * FROM role_panels WHERE message_id = $1")
                .bind(message_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;
        match panel {
            Some(p) => {
                let entries = sqlx::query_as::<_, EntryRow>(
                    "SELECT * FROM role_panel_entries WHERE panel_id = $1 ORDER BY position ASC",
                )
                .bind(p.id)
                .fetch_all(&self.pool)
                .await
                .map_err(pg_err)?;
                Ok(Some(RolePanelDetail {
                    panel: p.into(),
                    entries: entries.into_iter().map(RolePanelEntry::from).collect(),
                }))
            }
            None => Ok(None),
        }
    }

    async fn find_panels_by_guild(&self, guild_id: &str) -> Result<Vec<RolePanel>, DomainError> {
        let rows = sqlx::query_as::<_, PanelRow>(
            "SELECT * FROM role_panels WHERE guild_id = $1 ORDER BY created_at DESC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(RolePanel::from).collect())
    }

    async fn update_message_id(&self, panel_id: &str, message_id: &str) -> Result<(), DomainError> {
        let id: Uuid = panel_id
            .parse()
            .map_err(|_| DomainError::ValidationError("ID invalide".into()))?;
        sqlx::query("UPDATE role_panels SET message_id = $1, updated_at = NOW() WHERE id = $2")
            .bind(message_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn delete_panel(&self, panel_id: &str) -> Result<(), DomainError> {
        let id: Uuid = panel_id
            .parse()
            .map_err(|_| DomainError::ValidationError("ID invalide".into()))?;
        sqlx::query("DELETE FROM role_panels WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn find_auto_roles(&self, guild_id: &str) -> Result<Vec<AutoRole>, DomainError> {
        let rows = sqlx::query_as::<_, AutoRoleRow>("SELECT * FROM auto_roles WHERE guild_id = $1")
            .bind(guild_id)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(AutoRole::from).collect())
    }

    async fn save_auto_role(&self, ar: &AutoRole) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO auto_roles (id, guild_id, role_id, role_name, delay_secs, enabled) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (guild_id, role_id) DO UPDATE SET role_name=$4, delay_secs=$5, enabled=$6"
        )
        .bind(ar.id).bind(ar.guild_id.as_str()).bind(ar.role_id.as_str()).bind(&ar.role_name).bind(ar.delay_secs).bind(ar.enabled)
        .execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }

    async fn delete_auto_role(&self, guild_id: &str, role_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM auto_roles WHERE guild_id = $1 AND role_id = $2")
            .bind(guild_id)
            .bind(role_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }
}
