//! Client API du module security.
//!
//! Migration gRPC complete :
//! - Security events (report + list) -> `SecurityService`
//! - Members CRUD (sync, register, remove, update) -> `MembersService`

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::grpc_client::SentinelGrpcClient;

use platform_proto::sentinel::members::v1 as proto_members;
use platform_proto::sentinel::security::v1 as proto_security;
use platform_proto::sentinel::security_state::v1 as proto_state;

/// Ce que le serveur a decide pour un membre en attente d'acceptation du
/// reglement : la duree qu'il a devant lui, le rappel prevu, et si une
/// expulsion suivra.
///
/// Les valeurs par defaut ne servent qu'au cas ou l'API est injoignable : le
/// membre est deja en quarantaine cote Discord, mieux vaut un message qui
/// annonce une duree plausible qu'aucun message du tout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReglementApplique {
    pub timeout_secs: i64,
    pub reminder_secs: i64,
    pub kick_enabled: bool,
}

impl Default for ReglementApplique {
    fn default() -> Self {
        Self {
            timeout_secs: 24 * 3600,
            reminder_secs: 3600,
            kick_enabled: true,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SecurityEvent {
    pub guild_id: String,
    pub event_type: String,
    pub severity: String,
    pub description: String,
    pub user_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberPayload {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub roles: serde_json::Value,
    pub joined_at: Option<DateTime<Utc>>,
    pub account_created: Option<DateTime<Utc>>,
    pub is_bot: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SyncMembersPayload {
    pub guild_id: String,
    pub members: Vec<MemberPayload>,
}

#[derive(Debug, Serialize)]
pub struct UpdateMemberPayload {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub roles: Option<serde_json::Value>,
}

pub struct ApiClient {
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { grpc }
    }

    // ── Security events (gRPC) ──

    pub async fn list_events(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<serde_json::Value>, String> {
        let req = proto_security::ListEventsRequest {
            guild_id: Some(guild_id.to_string()),
        };
        let list = crate::grpc_call!(self.grpc, security, list_events, req)?;
        // Le proto ne porte pas de champ `limit` : on tronque cote client.
        // Les evenements sont supposes etre retournes les plus recents d'abord.
        Ok(list
            .events
            .into_iter()
            .take(limit as usize)
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "guild_id": e.guild_id,
                    "event_type": e.event_type,
                    "severity": e.severity,
                    "description": e.description,
                    "user_ids": e.user_ids,
                    "created_at": e.created_at,
                })
            })
            .collect())
    }

    pub async fn report_event(&self, event: &SecurityEvent) -> Result<(), String> {
        let req = proto_security::ReportEventRequest {
            guild_id: event.guild_id.clone(),
            event_type: event.event_type.clone(),
            severity: event.severity.clone(),
            description: event.description.clone(),
            user_ids: event.user_ids.clone(),
        };
        crate::grpc_call!(@unit self.grpc, security, report_event, req)
    }

    // ── Etat de securite : quarantaine / slowmode / lockdown (gRPC) ──
    //
    // Miroir de persistance durable (SecurityStateService). Le tracker RAM du
    // bot reste source de verite pour les roles/overwrites a restaurer ; ces
    // appels ne persistent que le timer + les states pour survivre a un reboot.

    /// Rehydratation au demarrage : quarantaines encore actives `(guild_id, user_id)`.
    pub async fn list_active_quarantines(&self) -> Result<Vec<(String, String)>, String> {
        let req = proto_state::ListActiveQuarantinesRequest {};
        let list = crate::grpc_call!(self.grpc, security_state, list_active_quarantines, req)?;
        Ok(list
            .entries
            .into_iter()
            .map(|e| (e.guild_id, e.user_id))
            .collect())
    }

    /// Enregistre (UPSERT idempotent) une quarantaine et retourne le reglement
    /// que le SERVEUR a applique.
    ///
    /// Le delai n'est pas envoye : il appartient a la configuration de la
    /// guilde. Le bot le decouvre ici, et s'en sert pour ecrire un message qui
    /// annonce la duree reelle.
    pub async fn mark_quarantine(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<ReglementApplique, String> {
        let req = proto_state::MarkQuarantineRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            // Zero : aucun delai impose, le reglage de la guilde fait foi.
            timeout_secs: 0,
        };
        let ack = crate::grpc_call!(self.grpc, security_state, mark_quarantine, req)?;
        Ok(ReglementApplique {
            timeout_secs: ack.timeout_secs,
            reminder_secs: ack.reminder_secs,
            kick_enabled: ack.kick_enabled,
        })
    }

    /// Lit le reglement applique par la guilde, sans rien modifier.
    ///
    /// Passer par `mark_quarantine` aurait relance le compte a rebours du
    /// membre : un clic sur un bouton perime lui aurait redonne un delai
    /// complet, indefiniment.
    pub async fn quarantine_settings(&self, guild_id: &str) -> Result<ReglementApplique, String> {
        let req = proto_state::GetQuarantineSettingsRequest {
            guild_id: guild_id.to_string(),
        };
        let ack = crate::grpc_call!(self.grpc, security_state, get_quarantine_settings, req)?;
        Ok(ReglementApplique {
            timeout_secs: ack.timeout_secs,
            reminder_secs: ack.reminder_secs,
            kick_enabled: ack.kick_enabled,
        })
    }

    /// Leve la quarantaine (captcha valide ou retrait admin). Idempotent.
    pub async fn lift_quarantine(&self, guild_id: &str, user_id: &str) -> Result<(), String> {
        let req = proto_state::LiftQuarantineRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        crate::grpc_call!(@unit self.grpc, security_state, lift_quarantine, req)
    }

    /// Enregistre (UPSERT idempotent) un slowmode actif. `previous_states` est le
    /// JSON des rates d'origine par salon (a restaurer a l'expiration).
    pub async fn mark_slowmode(
        &self,
        guild_id: &str,
        previous_states: serde_json::Value,
        duration_secs: i64,
        imposed_rate: i32,
    ) -> Result<(), String> {
        let previous_states_json = serde_json::to_string(&previous_states)
            .map_err(|e| format!("serialisation previous_states: {e}"))?;
        let req = proto_state::MarkSlowmodeRequest {
            guild_id: guild_id.to_string(),
            previous_states_json,
            duration_secs,
            imposed_rate,
        };
        crate::grpc_call!(@unit self.grpc, security_state, mark_slowmode, req)
    }

    /// Enregistre (UPSERT idempotent) un lockdown actif. `saved_states` est le
    /// JSON des overwrites originaux par salon (a restaurer a l'expiration).
    pub async fn mark_lockdown(
        &self,
        guild_id: &str,
        saved_states: serde_json::Value,
        duration_secs: i64,
    ) -> Result<(), String> {
        let saved_states_json = serde_json::to_string(&saved_states)
            .map_err(|e| format!("serialisation saved_states: {e}"))?;
        let req = proto_state::MarkLockdownRequest {
            guild_id: guild_id.to_string(),
            saved_states_json,
            duration_secs,
        };
        crate::grpc_call!(@unit self.grpc, security_state, mark_lockdown, req)
    }

    // ── Members CRUD (gRPC) ──

    pub async fn sync_members(&self, payload: &SyncMembersPayload) -> Result<(), String> {
        let req = proto_members::SyncMembersRequest {
            guild_id: payload.guild_id.clone(),
            members: payload
                .members
                .iter()
                .map(member_payload_to_proto)
                .collect::<Result<Vec<_>, String>>()?,
        };
        crate::grpc_call!(@unit self.grpc, members, sync_members, req)
    }

    pub async fn register_member(&self, member: &MemberPayload) -> Result<(), String> {
        let req = proto_members::RegisterMemberRequest {
            member: Some(member_payload_to_proto(member)?),
        };
        crate::grpc_call!(@unit self.grpc, members, register_member, req)
    }

    pub async fn update_member(
        &self,
        guild_id: &str,
        user_id: &str,
        payload: &UpdateMemberPayload,
    ) -> Result<(), String> {
        let roles_json = match &payload.roles {
            Some(v) => {
                Some(serde_json::to_string(v).map_err(|e| format!("serialisation roles: {e}"))?)
            }
            None => None,
        };
        let req = proto_members::UpdateMemberRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: payload.username.clone(),
            display_name: payload.display_name.clone(),
            avatar: payload.avatar.clone(),
            roles_json,
        };
        crate::grpc_call!(@unit self.grpc, members, update_member, req)
    }

    // ── Analyse nouveau membre (gRPC) ──

    pub async fn analyze_new_member(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        has_avatar: bool,
        account_created_timestamp: i64,
        is_bot: bool,
        recent_joins: Vec<RecentJoinEntry>,
        is_velocity_raid: bool,
    ) -> Result<SecurityDecisionResponse, String> {
        let req = proto_security::AnalyzeNewMemberRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            has_avatar,
            account_created_timestamp,
            is_bot,
            is_velocity_raid,
            recent_joins: recent_joins
                .into_iter()
                .map(|j| proto_security::RecentJoinEntry {
                    username: j.username,
                    has_avatar: j.has_avatar,
                    account_created_timestamp: j.account_created_timestamp,
                })
                .collect(),
        };
        let resp = crate::grpc_call!(self.grpc, security, analyze_new_member, req)?;
        Ok(SecurityDecisionResponse {
            is_raid: resp.is_raid,
            raid_score: resp.raid_score,
            is_suspicious_account: resp.is_suspicious_account,
            is_alt_account: resp.is_alt_account,
            quarantine: resp.quarantine,
            send_captcha: resp.send_captcha,
            activate_lockdown: resp.activate_lockdown,
            slowmode_secs: resp.slowmode_secs,
            suggest_only: resp.suggest_only,
            event_type: resp.event_type,
            event_description: resp.event_description,
        })
    }
}

// ── DTOs ──

#[derive(Debug, Clone)]
pub struct RecentJoinEntry {
    pub username: String,
    pub has_avatar: bool,
    pub account_created_timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct SecurityDecisionResponse {
    pub is_raid: bool,
    pub raid_score: u32,
    pub is_suspicious_account: bool,
    pub is_alt_account: bool,
    pub quarantine: bool,
    pub send_captcha: bool,
    pub activate_lockdown: bool,
    pub slowmode_secs: u32,
    /// La reponse guild-wide doit etre suggeree au staff (pas appliquee auto).
    pub suggest_only: bool,
    pub event_type: String,
    pub event_description: String,
}

fn member_payload_to_proto(p: &MemberPayload) -> Result<proto_members::GuildMember, String> {
    let roles_json =
        serde_json::to_string(&p.roles).map_err(|e| format!("serialisation roles: {e}"))?;
    Ok(proto_members::GuildMember {
        guild_id: p.guild_id.clone(),
        user_id: p.user_id.clone(),
        username: p.username.clone(),
        display_name: p.display_name.clone(),
        avatar: p.avatar.clone(),
        roles_json,
        joined_at: p.joined_at.map(|d| d.to_rfc3339()),
        account_created: p.account_created.map(|d| d.to_rfc3339()),
        is_bot: p.is_bot,
        last_seen_at: p.last_seen_at.map(|d| d.to_rfc3339()),
    })
}

use crate::shared::grpc_client::grpc_err_to_string;
