//! Client API du community-bot.
//!
//! Phase 7A — Migration gRPC :
//! - Role panels (5 methodes : get_panel, get_panel_by_message, list_panels,
//!   set_message_id, get_auto_roles) -> `RolePanelsService` partage avec
//!   roles-bot.
//! - Sponsorships et temp-roles (write/list/delete) restent en HTTP : pas de
//!   use case unifie cote API, repos directs.
//!
//! Surface publique inchangee.

use std::sync::Arc;

use crate::shared::grpc_client::SentinelGrpcClient;
use serde::{Deserialize, Serialize};

use platform_proto::sentinel::community::v1 as proto_community;
use platform_proto::sentinel::roles::v1 as proto;

// ── DTOs (surface inchangee) ──

/// Decision d'eligibilite renvoyee par l'API (role ou parrainage).
#[derive(Debug)]
pub struct EligibilityDecision {
    pub allowed: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TempRoleApiEntry {
    pub guild_id: String,
    pub user_id: String,
    pub role_id: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
pub struct RolePanelDetail {
    pub panel: RolePanel,
    pub entries: Vec<RolePanelEntry>,
}

#[derive(Debug, Deserialize)]
pub struct RolePanel {
    pub id: String,
    pub message_id: Option<String>,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct RolePanelEntry {
    pub role_id: String,
    pub emoji: Option<String>,
    pub label: String,
    pub style: String,
}

#[derive(Debug, Deserialize)]
pub struct AutoRole {
    pub role_id: String,
    pub delay_secs: i32,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct SyncRole {
    pub id: String,
    pub name: String,
    pub color: i32,
    pub position: i32,
    pub permissions: String,
    pub mentionable: bool,
    pub managed: bool,
    pub icon: Option<String>,
    pub member_count: i32,
}

#[derive(Clone)]
pub struct ApiClient {
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { grpc }
    }

    // ── Eligibilite (gRPC) — DECISION server-side ──

    /// Decide de l'eligibilite au role. Le bot fournit les donnees Discord
    /// (roles actuels + date de join) ; les prerequis vivent cote serveur.
    pub async fn check_role_eligibility(
        &self,
        guild_id: &str,
        role_id: u64,
        user_roles: Vec<u64>,
        joined_at_unix: Option<i64>,
    ) -> Result<EligibilityDecision, String> {
        let req = proto_community::CheckRoleEligibilityRequest {
            guild_id: guild_id.to_string(),
            role_id,
            user_roles,
            joined_at_unix,
        };
        let d = crate::grpc_call!(self.grpc, community, check_role_eligibility, req)?;
        Ok(EligibilityDecision {
            allowed: d.allowed,
            reason: d.reason,
        })
    }

    /// Valide un parrainage (anti-self + seuils). Le bot fournit les
    /// `joined_at` Discord ; la regle et les seuils vivent cote serveur.
    pub async fn validate_sponsorship_eligibility(
        &self,
        guild_id: &str,
        sponsor_id: u64,
        sponsored_id: u64,
        sponsor_joined_at_unix: Option<i64>,
        sponsored_joined_at_unix: Option<i64>,
    ) -> Result<EligibilityDecision, String> {
        let req = proto_community::ValidateSponsorshipRequest {
            guild_id: guild_id.to_string(),
            sponsor_id,
            sponsored_id,
            sponsor_joined_at_unix,
            sponsored_joined_at_unix,
        };
        let d = crate::grpc_call!(self.grpc, community, validate_sponsorship_eligibility, req)?;
        Ok(EligibilityDecision {
            allowed: d.allowed,
            reason: d.reason,
        })
    }

    // ── Role panels (gRPC) ──

    pub async fn get_auto_roles(&self, guild_id: &str) -> Result<Vec<AutoRole>, String> {
        let req = proto::ListAutoRolesRequest {
            guild_id: guild_id.to_string(),
        };
        let list = crate::grpc_call!(self.grpc, role_panels, list_auto_roles, req)?;
        Ok(list.roles.into_iter().map(proto_auto_role_to_dto).collect())
    }

    pub async fn set_message_id(&self, panel_id: &str, message_id: &str) -> Result<(), String> {
        let req = proto::SetMessageIdRequest {
            panel_id: panel_id.to_string(),
            message_id: message_id.to_string(),
        };
        crate::grpc_call!(@unit self.grpc, role_panels, set_message_id, req)
    }

    pub async fn list_panels(&self, guild_id: &str) -> Result<Vec<RolePanel>, String> {
        let req = proto::ListPanelsRequest {
            guild_id: guild_id.to_string(),
        };
        let list = crate::grpc_call!(self.grpc, role_panels, list_panels, req)?;
        Ok(list.panels.into_iter().map(proto_panel_to_dto).collect())
    }

    pub async fn get_panel(&self, panel_id: &str) -> Result<Option<RolePanelDetail>, String> {
        let req = proto::GetPanelRequest {
            panel_id: panel_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, role_panels, get_panel, req)?;
        Ok(resp.panel.map(proto_detail_to_dto))
    }

    // ── Phase 7A.opt F.3 — Sponsorships + Temp Roles en gRPC ──

    /// gRPC `CommunityService.CreateSponsorship`.
    /// Retourne Result pour permettre au caller de rollback en cas d'echec.
    pub async fn create_sponsorship(
        &self,
        guild_id: &str,
        sponsor_id: &str,
        sponsored_id: &str,
    ) -> Result<(), String> {
        let req = proto_community::CreateSponsorshipRequest {
            guild_id: guild_id.to_string(),
            sponsor_id: sponsor_id.to_string(),
            sponsored_id: sponsored_id.to_string(),
        };
        let mut client = self.grpc.community();
        self.grpc
            .guarded(|| async move { client.create_sponsorship(req).await.map(|_| ()) })
            .await
            .map_err(|e| {
                tracing::warn!(error = ?e, "Failed to create sponsorship");
                grpc_err_to_string(e)
            })
    }

    /// gRPC `CommunityService.CreateTempRole`.
    /// Retourne Result pour permettre au caller de rollback (ex: ne pas
    /// assigner le role Discord si la persistance echoue).
    pub async fn create_temp_role(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
        expires_at: &str,
    ) -> Result<(), String> {
        let req = proto_community::CreateTempRoleRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            role_id: role_id.to_string(),
            expires_at: expires_at.to_string(),
        };
        let mut client = self.grpc.community();
        self.grpc
            .guarded(|| async move { client.create_temp_role(req).await.map(|_| ()) })
            .await
            .map_err(|e| {
                tracing::warn!(error = ?e, "Failed to create temp role");
                grpc_err_to_string(e)
            })
    }

    /// gRPC `CommunityService.ListTempRoles`.
    pub async fn list_temp_roles(&self, guild_id: &str) -> Result<Vec<TempRoleApiEntry>, String> {
        let req = proto_community::ListTempRolesRequest {
            guild_id: guild_id.to_string(),
        };
        let list = crate::grpc_call!(self.grpc, community, list_temp_roles, req)?;
        Ok(list
            .roles
            .into_iter()
            .map(|r| TempRoleApiEntry {
                guild_id: r.guild_id,
                user_id: r.user_id,
                role_id: r.role_id,
                expires_at: r.expires_at,
            })
            .collect())
    }

    /// gRPC `CommunityService.DeleteTempRole`.
    /// Synchronise les roles Discord d'une guild vers l'API (depuis roles-bot).
    pub async fn sync_discord_roles(
        &self,
        guild_id: &str,
        roles: Vec<SyncRole>,
    ) -> Result<(), String> {
        let req = proto::SyncDiscordRolesRequest {
            guild_id: guild_id.to_string(),
            roles: roles
                .into_iter()
                .map(|r| proto::SyncDiscordRole {
                    id: r.id,
                    name: r.name,
                    color: r.color,
                    position: r.position,
                    permissions: r.permissions,
                    mentionable: r.mentionable,
                    managed: r.managed,
                    icon: r.icon,
                    member_count: r.member_count,
                })
                .collect(),
        };
        crate::grpc_call!(@unit self.grpc, role_panels, sync_discord_roles, req)
    }

    pub async fn delete_temp_role(&self, guild_id: &str, user_id: &str, role_id: &str) {
        let req = proto_community::DeleteTempRoleRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            role_id: role_id.to_string(),
        };
        if let Err(e) = crate::grpc_call!(@raw_unit self.grpc, community, delete_temp_role, req) {
            tracing::warn!(error = ?e, "Failed to delete temp role");
        }
    }
}

// ── Helpers proto -> DTO ──

fn proto_panel_to_dto(p: proto::RolePanel) -> RolePanel {
    RolePanel {
        id: p.id,
        message_id: p.message_id,
        title: p.title,
        description: p.description,
    }
}

fn proto_entry_to_dto(e: proto::RolePanelEntry) -> RolePanelEntry {
    RolePanelEntry {
        role_id: e.role_id,
        emoji: e.emoji,
        label: e.label,
        style: e.style,
    }
}

fn proto_detail_to_dto(d: proto::RolePanelDetail) -> RolePanelDetail {
    RolePanelDetail {
        panel: d.panel.map(proto_panel_to_dto).unwrap_or(RolePanel {
            id: String::new(),
            message_id: None,
            title: String::new(),
            description: String::new(),
        }),
        entries: d.entries.into_iter().map(proto_entry_to_dto).collect(),
    }
}

fn proto_auto_role_to_dto(r: proto::AutoRole) -> AutoRole {
    AutoRole {
        role_id: r.role_id,
        delay_secs: r.delay_secs,
        enabled: r.enabled,
    }
}

use crate::shared::grpc_client::grpc_err_to_string;
