//! Implementation gRPC du `StatsService`. Wrappe le use-case
//! `ManageStatsUseCase` deja utilise par les handlers HTTP — meme broadcast WS.

use std::sync::Arc;

use platform_proto::sentinel::stats::v1 as proto;
use platform_proto::sentinel::stats::v1::stats_service_server::StatsService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use crate::sentinel::adapters::outbound::ws::broadcaster::EventBroadcaster;
use platform_core::sentinel::domain::entities::audit::user_stats::GuildStatsOverview;
use platform_core::sentinel::domain::entities::audit::user_stats::UserStats;
use platform_core::sentinel::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use platform_core::sentinel::ports::inbound::audit::manage_stats::RecordMessagesCommand;
use platform_core::sentinel::ports::inbound::audit::manage_stats::RecordVoiceCommand;
pub struct StatsGrpc {
    pub stats_uc: Arc<dyn ManageStatsUseCase>,
    pub broadcaster: Arc<EventBroadcaster>,
}

#[tonic::async_trait]
impl StatsService for StatsGrpc {
    async fn record_messages(
        &self,
        request: Request<proto::RecordMessagesRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        let guild_id = req.guild_id.clone();
        let user_id = req.user_id.clone();
        let count = req.count;

        self.stats_uc
            .record_messages(RecordMessagesCommand {
                guild_id: req.guild_id.into(),
                user_id: req.user_id.into(),
                username: req.username,
                count: req.count,
            })
            .await
            .map_err(domain_to_status)?;

        self.broadcaster.broadcast(
            "stats_messages_recorded",
            serde_json::json!({ "guild_id": &guild_id, "user_id": &user_id, "count": count }),
        );
        Ok(Response::new(proto::Empty {}))
    }

    async fn record_voice(
        &self,
        request: Request<proto::RecordVoiceRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        let guild_id = req.guild_id.clone();
        let user_id = req.user_id.clone();
        let seconds = req.seconds;

        self.stats_uc
            .record_voice(RecordVoiceCommand {
                guild_id: req.guild_id.into(),
                user_id: req.user_id.into(),
                username: req.username,
                seconds: req.seconds,
                channel_id: req.channel_id.into(),
                channel_name: req.channel_name,
            })
            .await
            .map_err(domain_to_status)?;

        self.broadcaster.broadcast(
            "stats_voice_recorded",
            serde_json::json!({ "guild_id": &guild_id, "user_id": &user_id, "seconds": seconds }),
        );
        Ok(Response::new(proto::Empty {}))
    }

    async fn get_user_stats(
        &self,
        request: Request<proto::GetUserStatsRequest>,
    ) -> Result<Response<proto::GetUserStatsResponse>, Status> {
        let req = request.into_inner();
        let stats = self
            .stats_uc
            .get_user_stats(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::GetUserStatsResponse {
            stats: stats.map(user_stats_to_proto),
        }))
    }

    async fn get_guild_overview(
        &self,
        request: Request<proto::GetGuildOverviewRequest>,
    ) -> Result<Response<proto::GuildOverview>, Status> {
        let req = request.into_inner();
        let overview = self
            .stats_uc
            .get_guild_overview(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(guild_overview_to_proto(overview)))
    }

    async fn get_leaderboard(
        &self,
        request: Request<proto::GetLeaderboardRequest>,
    ) -> Result<Response<proto::UserStatsList>, Status> {
        let req = request.into_inner();
        let limit = if req.limit == 0 {
            10
        } else {
            req.limit.min(50)
        };
        let users = self
            .stats_uc
            .get_leaderboard(&req.guild_id, limit)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::UserStatsList {
            users: users.into_iter().map(user_stats_to_proto).collect(),
        }))
    }
}

fn user_stats_to_proto(u: UserStats) -> proto::UserStats {
    proto::UserStats {
        id: u.id.to_string(),
        guild_id: u.guild_id.into(),
        user_id: u.user_id.into(),
        username: u.username,
        message_count: u.message_count,
        voice_seconds: u.voice_seconds,
        updated_at: u.updated_at.to_rfc3339(),
    }
}

fn guild_overview_to_proto(o: GuildStatsOverview) -> proto::GuildOverview {
    proto::GuildOverview {
        guild_id: o.guild_id.into(),
        total_messages: o.total_messages,
        total_voice_seconds: o.total_voice_seconds,
        active_members: o.active_members,
        total_infractions: o.total_infractions,
        total_warns: o.total_warns,
        total_mutes: o.total_mutes,
        total_bans: o.total_bans,
        top_members: o.top_members.into_iter().map(user_stats_to_proto).collect(),
    }
}

#[cfg(test)]
#[path = "tests/stats.rs"]
mod tests;
