//! Client du module guild_backup, entierement en gRPC.
//!
//! Le bot appelle l'API en interne (Bearer API key, pas de `X-Discord-Token`) :
//! le controle d'acces a la capture et a la restauration est assure cote API
//! par les middlewares du routeur, avant meme la publication de l'event.
//!
//! Le `GuildSnapshot` transite serialise en JSON — voir `guild_backup.proto`
//! pour le raisonnement (payload document, stocke en JSONB, jamais inspecte
//! champ par champ en transit).

use std::sync::Arc;

use serde::Deserialize;

use crate::shared::grpc_client::{grpc_err_to_string, SentinelGrpcClient};
use platform_core::sentinel::domain::entities::guild_backup::snapshot::GuildSnapshot;
use platform_proto::sentinel::guild_backup::v1 as proto;

/// Identifiant d'une sauvegarde stockee (UUID renvoye par l'API).
pub type SnapshotId = String;

/// Resume leger d'une sauvegarde (liste sans le payload complet).
#[derive(Debug, Clone, Deserialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub label: String,
    pub created_at: String,
    pub role_count: u32,
    pub channel_count: u32,
}

/// Roles a re-attribuer a un membre absent au moment du restore.
#[derive(Debug)]
pub struct PendingRoleGrant {
    pub user_id: String,
    pub role_ids: Vec<String>,
}

/// Stocke une capture. Renvoie son identifiant.
pub async fn store_snapshot(
    grpc: &Arc<SentinelGrpcClient>,
    guild_id: &str,
    snapshot: &GuildSnapshot,
) -> Result<SnapshotId, String> {
    let snapshot_json =
        serde_json::to_string(snapshot).map_err(|e| format!("Serialisation capture : {e}"))?;
    let req = proto::StoreSnapshotRequest {
        guild_id: guild_id.to_string(),
        snapshot_json,
    };
    let resp = crate::grpc_call!(grpc, guild_backup, store_snapshot, req)?;
    Ok(resp.snapshot_id)
}

/// Liste les captures d'un serveur (resumes, sans le payload).
pub async fn list_snapshots(
    grpc: &Arc<SentinelGrpcClient>,
    guild_id: &str,
) -> Result<Vec<SnapshotSummary>, String> {
    let req = proto::ListSnapshotsRequest {
        guild_id: guild_id.to_string(),
    };
    let resp = crate::grpc_call!(grpc, guild_backup, list_snapshots, req)?;
    Ok(resp
        .snapshots
        .into_iter()
        .map(|s| SnapshotSummary {
            id: s.id,
            label: s.label,
            created_at: s.created_at,
            role_count: s.role_count,
            channel_count: s.channel_count,
        })
        .collect())
}

/// Charge une capture complete.
pub async fn get_snapshot(
    grpc: &Arc<SentinelGrpcClient>,
    snapshot_id: &str,
) -> Result<GuildSnapshot, String> {
    let req = proto::GetSnapshotRequest {
        snapshot_id: snapshot_id.to_string(),
    };
    let resp = crate::grpc_call!(grpc, guild_backup, get_snapshot, req)?;
    serde_json::from_str(&resp.snapshot_json)
        .map_err(|e| format!("Capture illisible (schema incompatible ?) : {e}"))
}

/// Supprime une capture.
pub async fn delete_snapshot(
    grpc: &Arc<SentinelGrpcClient>,
    snapshot_id: &str,
) -> Result<(), String> {
    let req = proto::DeleteSnapshotRequest {
        snapshot_id: snapshot_id.to_string(),
    };
    crate::grpc_call!(@unit grpc, guild_backup, delete_snapshot, req)
}

/// Enregistre les roles a re-attribuer aux membres absents. Renvoie le nombre
/// d'entrees ecrites.
pub async fn save_pending_roles(
    grpc: &Arc<SentinelGrpcClient>,
    guild_id: &str,
    grants: &[PendingRoleGrant],
) -> Result<u64, String> {
    let req = proto::SavePendingRolesRequest {
        guild_id: guild_id.to_string(),
        grants: grants
            .iter()
            .map(|g| proto::PendingRoleGrant {
                user_id: g.user_id.clone(),
                role_ids: g.role_ids.clone(),
            })
            .collect(),
    };
    let resp = crate::grpc_call!(grpc, guild_backup, save_pending_roles, req)?;
    Ok(resp.saved)
}

/// Lit ET supprime (atomique) les roles en attente d'un membre. Vecteur vide
/// si aucun : c'est le cas de la quasi-totalite des arrivees.
pub async fn consume_pending_roles(
    grpc: &Arc<SentinelGrpcClient>,
    guild_id: &str,
    user_id: &str,
) -> Result<Vec<String>, String> {
    let req = proto::ConsumePendingRolesRequest {
        guild_id: guild_id.to_string(),
        user_id: user_id.to_string(),
    };
    let resp = crate::grpc_call!(grpc, guild_backup, consume_pending_roles, req)?;
    Ok(resp.role_ids)
}

/// Purge les roles en attente d'un serveur (repartir propre avant un nouveau
/// restore). Best-effort cote appelant.
pub async fn clear_pending_roles(
    grpc: &Arc<SentinelGrpcClient>,
    guild_id: &str,
) -> Result<(), String> {
    let req = proto::ClearPendingRolesRequest {
        guild_id: guild_id.to_string(),
    };
    crate::grpc_call!(@unit grpc, guild_backup, clear_pending_roles, req)
}
