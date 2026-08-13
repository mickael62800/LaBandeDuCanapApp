//! gRPC handler de la sauvegarde / restauration de serveur.
//!
//! Miroir des routes HTTP `/api/guild-backup/*` appelees par le bot. Le
//! dashboard web garde ses routes HTTP : il declenche capture et restauration
//! via des events Redis, et affiche la liste des captures.
//!
//! Le `GuildSnapshot` transite en JSON (cf. `guild_backup.proto` pour le
//! raisonnement) : ce handler ne fait que le (de)serialiser aux bornes.

use std::sync::Arc;

use platform_proto::sentinel::guild_backup::v1 as proto;
use platform_proto::sentinel::guild_backup::v1::guild_backup_service_server::GuildBackupService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use platform_core::sentinel::domain::entities::guild_backup::pending_role_grant::PendingRoleGrant;
use platform_core::sentinel::domain::entities::guild_backup::snapshot::GuildSnapshot;
use platform_core::sentinel::ports::inbound::guild_backup::manage_pending_role_grants::ManagePendingRoleGrantsUseCase;
use platform_core::sentinel::ports::inbound::guild_backup::manage_snapshots::ManageGuildSnapshotsUseCase;

pub struct GuildBackupGrpc {
    pub snapshots_uc: Arc<dyn ManageGuildSnapshotsUseCase>,
    pub pending_role_grants_uc: Arc<dyn ManagePendingRoleGrantsUseCase>,
}

/// Parse un identifiant de capture, en refusant proprement un UUID malforme.
fn parse_snapshot_id(raw: &str) -> Result<uuid::Uuid, Status> {
    raw.parse()
        .map_err(|_| Status::invalid_argument("snapshot_id doit etre un UUID"))
}

#[tonic::async_trait]
impl GuildBackupService for GuildBackupGrpc {
    async fn store_snapshot(
        &self,
        request: Request<proto::StoreSnapshotRequest>,
    ) -> Result<Response<proto::StoreSnapshotResponse>, Status> {
        let req = request.into_inner();
        if req.guild_id.is_empty() {
            return Err(Status::invalid_argument("guild_id requis"));
        }
        // Un payload illisible est une erreur de l'appelant, pas du serveur :
        // `invalid_argument` et non `internal`.
        let snapshot: GuildSnapshot = serde_json::from_str(&req.snapshot_json)
            .map_err(|e| Status::invalid_argument(format!("snapshot_json invalide : {e}")))?;

        let id = self
            .snapshots_uc
            .store_snapshot(snapshot)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::StoreSnapshotResponse {
            snapshot_id: id.to_string(),
        }))
    }

    async fn list_snapshots(
        &self,
        request: Request<proto::ListSnapshotsRequest>,
    ) -> Result<Response<proto::ListSnapshotsResponse>, Status> {
        let req = request.into_inner();
        let summaries = self
            .snapshots_uc
            .list_snapshots(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ListSnapshotsResponse {
            snapshots: summaries
                .into_iter()
                .map(|s| proto::SnapshotSummary {
                    id: s.id.to_string(),
                    label: s.label,
                    created_at: s.created_at,
                    role_count: s.role_count,
                    channel_count: s.channel_count,
                })
                .collect(),
        }))
    }

    async fn get_snapshot(
        &self,
        request: Request<proto::GetSnapshotRequest>,
    ) -> Result<Response<proto::GetSnapshotResponse>, Status> {
        let req = request.into_inner();
        let id = parse_snapshot_id(&req.snapshot_id)?;
        let snapshot = self
            .snapshots_uc
            .get_snapshot(id)
            .await
            .map_err(domain_to_status)?;
        let snapshot_json = serde_json::to_string(&snapshot)
            .map_err(|e| Status::internal(format!("serialisation snapshot : {e}")))?;
        Ok(Response::new(proto::GetSnapshotResponse { snapshot_json }))
    }

    async fn delete_snapshot(
        &self,
        request: Request<proto::DeleteSnapshotRequest>,
    ) -> Result<Response<proto::DeleteSnapshotResponse>, Status> {
        let req = request.into_inner();
        let id = parse_snapshot_id(&req.snapshot_id)?;
        // `delete_snapshot` rend `false` si rien n'a ete supprime : c'est un
        // `not_found` pour l'appelant, qui croyait la capture existante.
        if self
            .snapshots_uc
            .delete_snapshot(id)
            .await
            .map_err(domain_to_status)?
        {
            Ok(Response::new(proto::DeleteSnapshotResponse {}))
        } else {
            Err(Status::not_found("capture introuvable"))
        }
    }

    async fn save_pending_roles(
        &self,
        request: Request<proto::SavePendingRolesRequest>,
    ) -> Result<Response<proto::SavePendingRolesResponse>, Status> {
        let req = request.into_inner();
        let grants: Vec<PendingRoleGrant> = req
            .grants
            .into_iter()
            .map(|g| PendingRoleGrant {
                guild_id: req.guild_id.clone(),
                user_id: g.user_id,
                role_ids: g.role_ids,
            })
            .collect();
        let saved = self
            .pending_role_grants_uc
            .save_grants(&req.guild_id, grants)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::SavePendingRolesResponse { saved }))
    }

    async fn consume_pending_roles(
        &self,
        request: Request<proto::ConsumePendingRolesRequest>,
    ) -> Result<Response<proto::ConsumePendingRolesResponse>, Status> {
        let req = request.into_inner();
        // Absence d'entree = liste vide, pas une erreur : la quasi-totalite des
        // membres qui rejoignent n'ont rien en attente.
        let role_ids = self
            .pending_role_grants_uc
            .take_grant(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?
            .unwrap_or_default();
        Ok(Response::new(proto::ConsumePendingRolesResponse {
            role_ids,
        }))
    }

    async fn clear_pending_roles(
        &self,
        request: Request<proto::ClearPendingRolesRequest>,
    ) -> Result<Response<proto::ClearPendingRolesResponse>, Status> {
        let req = request.into_inner();
        let cleared = self
            .pending_role_grants_uc
            .clear_guild(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ClearPendingRolesResponse { cleared }))
    }
}
