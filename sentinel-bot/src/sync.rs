//! Helpers de synchronisation Discord <-> Web (cf. SYNC_DISCORD_WEB_DESIGN.md).
//!
//! Quand un module poste un message Discord lie a une entite metier
//! (ban_proposal, ticket, roles_panel...), il appelle
//! `register_action_message` pour persister la correspondance
//! `action_id <-> (channel_id, message_id)` cote API. Les consommateurs
//! (edition/suppression depuis le web) retrouvent le message via
//! `list_action_messages`.
//!
//! Ressource de sync TRANSVERSE : passe par le `DiscordActionMessagesService`
//! (gRPC), pas par un ApiClient de module. Le register reste fire-and-forget
//! (si l'API est down, le post Discord reste valide, juste pas synchronise).

use std::sync::Arc;
use uuid::Uuid;

use crate::grpc_call;
use crate::shared::grpc_client::{grpc_err_to_string, SentinelGrpcClient};
use platform_proto::sentinel::discord_messages::v1 as proto;

/// Conventions de `kind` partagees avec le domain API
/// (sentinel-api/src/domain/entities/discord_action_message.rs::kinds).
pub mod kinds {
    pub const TICKET: &str = "ticket";
    pub const AUTOMOD_REVIEW: &str = "automod_review";
}

/// Un mapping `action_id -> message Discord` (sous-ensemble consomme par le bot).
#[derive(Debug, Clone)]
pub struct ActionMessageMapping {
    pub kind: String,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: String,
}

/// Enregistre une correspondance `action_id <-> message Discord`.
/// Fire-and-forget : log d'erreur mais ne propage pas. A appeler juste
/// apres avoir poste le message Discord.
pub async fn register_action_message(
    grpc: &Arc<SentinelGrpcClient>,
    action_id: Uuid,
    kind: &str,
    guild_id: &str,
    channel_id: &str,
    message_id: &str,
) {
    let req = proto::RegisterRequest {
        action_id: action_id.to_string(),
        kind: kind.to_string(),
        guild_id: guild_id.to_string(),
        channel_id: channel_id.to_string(),
        message_id: message_id.to_string(),
    };
    if let Err(e) = grpc_call!(@raw_unit grpc, discord_messages, register, req) {
        tracing::warn!(error = %e, %action_id, kind, "mapping discord_action_messages non enregistre");
    }
}

/// Liste toutes les representations Discord d'une entite metier (`action_id`).
pub async fn list_action_messages(
    grpc: &Arc<SentinelGrpcClient>,
    action_id: &str,
) -> Result<Vec<ActionMessageMapping>, String> {
    let req = proto::ListForActionRequest {
        action_id: action_id.to_string(),
    };
    let list = grpc_call!(grpc, discord_messages, list_for_action, req)?;
    Ok(list
        .messages
        .into_iter()
        .map(|m| ActionMessageMapping {
            kind: m.kind,
            guild_id: m.guild_id,
            channel_id: m.channel_id,
            message_id: m.message_id,
        })
        .collect())
}
