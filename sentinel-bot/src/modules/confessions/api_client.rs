//! Client gRPC du module confessions (`ConfessionsService`). Le module ne
//! garde en HTTP que l'ecriture de config transverse `/api/bots/config`
//! (`persist_confession_setting`), meme nature que `set_bot_config`.

use std::sync::Arc;

use crate::shared::grpc_client::{grpc_err_to_string, SentinelGrpcClient};
use platform_proto::sentinel::confessions::v1 as proto;

/// Vue bot d'une confession (sous-ensemble consomme).
#[derive(Debug, Clone, Default)]
pub struct ConfessionData {
    pub id: String,
    pub public_number: i64,
    pub author_user_id: String,
    pub channel_id: Option<String>,
    pub message_id: Option<String>,
    pub thread_id: Option<String>,
}

impl From<proto::Confession> for ConfessionData {
    fn from(c: proto::Confession) -> Self {
        Self {
            id: c.id,
            public_number: c.public_number as i64,
            author_user_id: c.author_user_id,
            channel_id: c.channel_id,
            message_id: c.message_id,
            thread_id: c.thread_id,
        }
    }
}

/// Vue bot d'une reponse (id + numero public).
#[derive(Debug, Clone)]
pub struct ReplyData {
    pub id: String,
    pub public_number: i64,
}

/// Vue bot de la config confessions (pour le salon + le message collant).
#[derive(Debug, Clone, Default)]
pub struct ConfessionConfigData {
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub panel_message_id: Option<String>,
}

pub struct ConfessionsApi {
    grpc: Arc<SentinelGrpcClient>,
}

impl ConfessionsApi {
    pub fn new(grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { grpc }
    }

    pub async fn create(
        &self,
        guild_id: &str,
        author_user_id: &str,
        content: &str,
    ) -> Result<ConfessionData, String> {
        let req = proto::CreateConfessionRequest {
            guild_id: guild_id.to_string(),
            author_user_id: author_user_id.to_string(),
            content: content.to_string(),
        };
        let c = crate::grpc_call!(self.grpc, confessions, create_confession, req)?;
        Ok(c.into())
    }

    pub async fn get(&self, id: &str) -> Result<ConfessionData, String> {
        let req = proto::GetConfessionRequest { id: id.to_string() };
        let c = crate::grpc_call!(self.grpc, confessions, get_confession, req)?;
        Ok(c.into())
    }

    pub async fn list(
        &self,
        guild_id: &str,
        limit: i64,
        include_deleted: bool,
    ) -> Result<Vec<ConfessionData>, String> {
        let req = proto::ListConfessionsRequest {
            guild_id: guild_id.to_string(),
            limit,
            include_deleted,
        };
        let list = crate::grpc_call!(self.grpc, confessions, list_confessions, req)?;
        Ok(list.confessions.into_iter().map(Into::into).collect())
    }

    pub async fn delete(
        &self,
        id: &str,
        deleted_by: &str,
        reason: Option<&str>,
    ) -> Result<(), String> {
        let req = proto::DeleteConfessionRequest {
            id: id.to_string(),
            deleted_by: deleted_by.to_string(),
            reason: reason.map(str::to_string),
        };
        crate::grpc_call!(@unit self.grpc, confessions, delete_confession, req)
    }

    pub async fn update_message_refs(
        &self,
        id: &str,
        message_id: &str,
        channel_id: &str,
        thread_id: Option<String>,
    ) -> Result<(), String> {
        let req = proto::UpdateMessageRefsRequest {
            id: id.to_string(),
            message_id: message_id.to_string(),
            channel_id: channel_id.to_string(),
            thread_id,
        };
        crate::grpc_call!(@unit self.grpc, confessions, update_message_refs, req)
    }

    pub async fn create_reply(
        &self,
        confession_id: &str,
        author_user_id: &str,
        content: &str,
        is_anonymous: bool,
    ) -> Result<ReplyData, String> {
        let req = proto::CreateReplyRequest {
            confession_id: confession_id.to_string(),
            author_user_id: author_user_id.to_string(),
            content: content.to_string(),
            is_anonymous,
        };
        let r = crate::grpc_call!(self.grpc, confessions, create_reply, req)?;
        Ok(ReplyData {
            id: r.id,
            public_number: r.public_number as i64,
        })
    }

    pub async fn update_reply_message_id(
        &self,
        reply_id: &str,
        message_id: &str,
    ) -> Result<(), String> {
        let req = proto::UpdateReplyMessageIdRequest {
            reply_id: reply_id.to_string(),
            message_id: message_id.to_string(),
        };
        crate::grpc_call!(@unit self.grpc, confessions, update_reply_message_id, req)
    }

    pub async fn create_report(
        &self,
        guild_id: &str,
        confession_id: Option<&str>,
        reply_id: Option<&str>,
        reporter_user_id: &str,
        reason: &str,
    ) -> Result<(), String> {
        let req = proto::CreateReportRequest {
            guild_id: guild_id.to_string(),
            confession_id: confession_id.map(str::to_string),
            reply_id: reply_id.map(str::to_string),
            reporter_user_id: reporter_user_id.to_string(),
            reason: reason.to_string(),
        };
        crate::grpc_call!(@unit self.grpc, confessions, create_report, req)
    }

    pub async fn get_config(&self, guild_id: &str) -> Result<ConfessionConfigData, String> {
        let req = proto::GetConfigRequest {
            guild_id: guild_id.to_string(),
        };
        let c = crate::grpc_call!(self.grpc, confessions, get_config, req)?;
        Ok(ConfessionConfigData {
            guild_id: c.guild_id,
            channel_id: c.channel_id,
            panel_message_id: c.panel_message_id,
        })
    }
}
