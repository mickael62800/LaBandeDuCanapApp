//! Client du module cleanup — purges administratives.
//!
//! Entierement en gRPC depuis la fin de la migration : les trois purges
//! passaient auparavant par `DELETE /api/purge/*` en HTTP. Les routes HTTP
//! restent en place pour le dashboard web, qui les utilise toujours.

use std::sync::Arc;

use crate::shared::grpc_client::{grpc_err_to_string, SentinelGrpcClient};
use platform_proto::sentinel::purge::v1 as proto;

pub struct ApiClient {
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { grpc }
    }

    /// Purge les infractions d'un serveur. `days = 0` = tout supprimer.
    pub async fn purge_infractions(&self, guild_id: &str, days: u64) -> Result<u64, String> {
        let req = proto::PurgeByDaysRequest {
            guild_id: guild_id.to_string(),
            days: days as i32,
        };
        let resp = crate::grpc_call!(self.grpc, purge, purge_infractions, req)?;
        Ok(resp.deleted)
    }

    /// Purge les audit-logs d'un serveur.
    pub async fn purge_audit_logs(&self, guild_id: &str, days: u64) -> Result<u64, String> {
        let req = proto::PurgeByDaysRequest {
            guild_id: guild_id.to_string(),
            days: days as i32,
        };
        let resp = crate::grpc_call!(self.grpc, purge, purge_audit_logs, req)?;
        Ok(resp.deleted)
    }

    /// Purge les logs applicatifs de TOUTES les guilds (portee globale).
    pub async fn purge_logs(&self, days: u64) -> Result<u64, String> {
        let req = proto::PurgeLogsRequest { days: days as i32 };
        let resp = crate::grpc_call!(self.grpc, purge, purge_logs, req)?;
        Ok(resp.deleted)
    }
}
