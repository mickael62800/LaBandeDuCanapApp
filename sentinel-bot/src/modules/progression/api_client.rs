//! Client API specifique au progression-bot.
//!
//! - Les endpoints **levels** (`record_text_activity`, `record_voice_activity`,
//!   `get_user_level`, `get_level_leaderboard`) et **stats** (`record_messages`,
//!   `record_voice`, `get_user_stats`, `get_guild_overview`, `get_leaderboard`)
//!   passent par gRPC via `SentinelGrpcClient`. Depuis le refactor P0, le bot
//!   n'envoie que des FAITS BRUTS : c'est l'API qui calcule tout l'XP.
//! - `force_monthly_ranking` et `count_user_infractions` passent aussi en
//!   gRPC, mais sur les services de LEUR domaine (`CommunityService` et
//!   `ModerationService`) : les greffer sur `ProgressionService` aurait fait
//!   du service un fourre-tout.

use std::sync::Arc;

use serde::Deserialize;

use crate::shared::grpc_client::{GrpcCallError, SentinelGrpcClient};

use platform_proto::sentinel::common::v1 as proto_common;
use platform_proto::sentinel::community::v1 as proto_community;
use platform_proto::sentinel::moderation::v1 as proto_mod;
use platform_proto::sentinel::progression::v1 as proto_prog;
use platform_proto::sentinel::stats::v1 as proto_stats;

// ── Response DTOs (surface publique inchangee) ──

/// Compteurs d'infractions d'un membre. `total` couvre toutes les natures,
/// y compris celles sans compteur dedie ici.
#[derive(Debug, Default)]
pub struct InfractionCounts {
    pub warns: u32,
    pub deletes: u32,
    pub mutes: u32,
    pub bans: u32,
    pub total: u32,
}

#[derive(Debug, Deserialize)]
pub struct UserStatsResponse {
    pub username: String,
    pub message_count: u64,
    pub voice_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct GuildOverviewResponse {
    pub total_messages: u64,
    pub total_voice_seconds: u64,
    pub active_members: u64,
    pub total_infractions: u64,
    pub total_warns: u64,
    pub total_mutes: u64,
    pub total_bans: u64,
}

#[derive(Debug, Deserialize)]
pub struct UserLevelResponse {
    pub user_id: String,
    pub username: String,
    pub xp: i64,
    pub level: i32,
    #[serde(default)]
    pub xp_text: i64,
    #[serde(default)]
    pub level_text: i32,
    #[serde(default)]
    pub xp_text_current: i64,
    #[serde(default)]
    pub xp_text_needed: i64,
    #[serde(default)]
    pub xp_voice: i64,
    #[serde(default)]
    pub level_voice: i32,
    #[serde(default)]
    pub xp_voice_current: i64,
    #[serde(default)]
    pub xp_voice_needed: i64,
}

/// Reponse a un fait d'activite (texte/vocal) : l'API a calcule tout l'XP.
///
/// Le bot ne lit que `skipped` (anti-spam XP) ; `xp_gained` et
/// `streak_current` sont renvoyes par l'API pour d'autres consommateurs et
/// conserves ici pour refleter le contrat gRPC.
#[derive(Debug)]
#[allow(dead_code)]
pub struct RecordActivityResponse {
    pub user: UserLevelResponse,
    pub leveled_up: bool,
    pub old_level_global: i32,
    pub xp_gained: i64,
    pub skipped: bool,
    pub streak_current: u32,
}

#[derive(Debug)]
pub struct RankingEntry {
    pub user_id: String,
    pub xp: i64,
}

#[derive(Debug)]
pub struct ForceRankingResponse {
    pub period_label: String,
    pub note: Option<String>,
    pub text: Vec<RankingEntry>,
    pub voice: Vec<RankingEntry>,
    pub global: Vec<RankingEntry>,
}

// ── Client ──

pub struct ApiClient {
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { grpc }
    }

    // ── Stats (gRPC) ──

    pub async fn record_messages(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        count: u64,
    ) -> Result<(), String> {
        let req = proto_stats::RecordMessagesRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            count,
        };
        crate::grpc_call!(@unit self.grpc, stats, record_messages, req)
    }

    pub async fn record_voice(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        seconds: u64,
        channel_id: &str,
        channel_name: &str,
    ) -> Result<(), String> {
        let req = proto_stats::RecordVoiceRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            seconds,
            channel_id: channel_id.to_string(),
            channel_name: channel_name.to_string(),
        };
        crate::grpc_call!(@unit self.grpc, stats, record_voice, req)
    }

    pub async fn get_user_stats(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserStatsResponse>, String> {
        let req = proto_stats::GetUserStatsRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, stats, get_user_stats, req)?;
        Ok(resp.stats.map(proto_user_stats_to_response))
    }

    pub async fn get_guild_overview(
        &self,
        guild_id: &str,
    ) -> Result<GuildOverviewResponse, String> {
        let req = proto_stats::GetGuildOverviewRequest {
            guild_id: guild_id.to_string(),
        };
        let overview = crate::grpc_call!(self.grpc, stats, get_guild_overview, req)?;
        Ok(GuildOverviewResponse {
            total_messages: overview.total_messages,
            total_voice_seconds: overview.total_voice_seconds,
            active_members: overview.active_members,
            total_infractions: overview.total_infractions,
            total_warns: overview.total_warns,
            total_mutes: overview.total_mutes,
            total_bans: overview.total_bans,
        })
    }

    pub async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<UserStatsResponse>, String> {
        let req = proto_stats::GetLeaderboardRequest {
            guild_id: guild_id.to_string(),
            limit,
        };
        let list = crate::grpc_call!(self.grpc, stats, get_leaderboard, req)?;
        Ok(list
            .users
            .into_iter()
            .map(proto_user_stats_to_response)
            .collect())
    }

    // ── Levels / XP (gRPC) ──

    /// Envoie un FAIT BRUT texte : "un message qualifiant a eu lieu". L'API
    /// calcule le montant d'XP (multiplicateurs channel/role, streak, cooldown).
    pub async fn record_text_activity(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        channel_id: u64,
        role_ids: &[u64],
    ) -> Result<RecordActivityResponse, String> {
        let req = proto_prog::RecordTextActivityRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            channel_id: channel_id.to_string(),
            role_ids: role_ids.iter().map(|r| r.to_string()).collect(),
        };
        let resp = crate::grpc_call!(self.grpc, progression, record_text_activity, req)?;
        Ok(proto_record_activity_to_response(resp))
    }

    /// Envoie un FAIT BRUT vocal : `seconds` secondes creditables dans le
    /// salon. L'API calcule le montant d'XP (multiplicateurs channel/role).
    pub async fn record_voice_activity(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        channel_id: u64,
        role_ids: &[u64],
        seconds: u64,
    ) -> Result<RecordActivityResponse, String> {
        let req = proto_prog::RecordVoiceActivityRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            channel_id: channel_id.to_string(),
            role_ids: role_ids.iter().map(|r| r.to_string()).collect(),
            seconds,
        };
        let resp = crate::grpc_call!(self.grpc, progression, record_voice_activity, req)?;
        Ok(proto_record_activity_to_response(resp))
    }

    pub async fn get_user_level(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserLevelResponse>, String> {
        let req = proto_prog::GetUserLevelRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let result = crate::grpc_call!(@raw self.grpc, progression, get_user_level, req);
        match result {
            Ok(level) => Ok(Some(proto_user_level_to_response(level))),
            Err(GrpcCallError::Status(s)) if s.code() == tonic::Code::NotFound => Ok(None),
            Err(e) => Err(grpc_err_to_string(e)),
        }
    }

    pub async fn get_level_leaderboard(
        &self,
        guild_id: &str,
        limit: u32,
        source: Option<&str>,
    ) -> Result<Vec<UserLevelResponse>, String> {
        let req = proto_prog::GetLeaderboardRequest {
            guild_id: guild_id.to_string(),
            limit: limit as i64,
            source: source
                .map(xp_source_str_to_proto)
                .unwrap_or(proto_common::XpSource::Unspecified as i32),
        };
        let board = crate::grpc_call!(self.grpc, progression, get_leaderboard, req)?;
        Ok(board
            .users
            .into_iter()
            .map(proto_user_level_to_response)
            .collect())
    }

    // ── Services voisins (gRPC) ──
    //
    // Le classement mensuel appartient au domaine community, les infractions
    // au domaine moderation : chacun est appele sur son propre service plutot
    // que greffe sur `ProgressionService`.

    /// Force le calcul du classement mensuel cote API (bypass des gates).
    /// Renvoie les donnees ; le bot fait le rendu + le post Discord.
    pub async fn force_monthly_ranking(
        &self,
        guild_id: &str,
        mois: &str,
    ) -> Result<ForceRankingResponse, String> {
        let req = proto_community::ForceMonthlyRankingRequest {
            guild_id: guild_id.to_string(),
            mois: mois.to_string(),
        };
        let r = crate::grpc_call!(self.grpc, community, force_monthly_ranking, req)?;
        Ok(ForceRankingResponse {
            period_label: r.period_label,
            note: r.note,
            text: ranking_entries(r.text),
            voice: ranking_entries(r.voice),
            global: ranking_entries(r.global),
        })
    }

    /// Compteurs d'infractions d'un membre, agreges cote serveur.
    ///
    /// L'ancienne route HTTP renvoyait le journal complet du serveur, que le
    /// bot filtrait ensuite en memoire pour afficher quatre nombres.
    pub async fn count_user_infractions(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<InfractionCounts, String> {
        let req = proto_mod::CountUserInfractionsRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let c = crate::grpc_call!(self.grpc, moderation, count_user_infractions, req)?;
        Ok(InfractionCounts {
            warns: c.warns,
            deletes: c.deletes,
            mutes: c.mutes,
            bans: c.bans,
            total: c.total,
        })
    }
}

fn ranking_entries(v: Vec<proto_community::RankingEntry>) -> Vec<RankingEntry> {
    v.into_iter()
        .map(|e| RankingEntry {
            user_id: e.user_id,
            xp: e.xp,
        })
        .collect()
}

// ── Helpers de conversion proto -> DTOs locaux ──

fn xp_source_str_to_proto(s: &str) -> i32 {
    match s {
        "voice" => proto_common::XpSource::Voice as i32,
        _ => proto_common::XpSource::Text as i32,
    }
}

fn proto_user_level_to_response(u: proto_prog::UserLevel) -> UserLevelResponse {
    UserLevelResponse {
        user_id: u.user_id,
        username: u.username,
        xp: u.xp,
        level: u.level,
        xp_text: u.xp_text,
        level_text: u.level_text,
        xp_text_current: u.xp_text_current,
        xp_text_needed: u.xp_text_needed,
        xp_voice: u.xp_voice,
        level_voice: u.level_voice,
        xp_voice_current: u.xp_voice_current,
        xp_voice_needed: u.xp_voice_needed,
    }
}

fn proto_record_activity_to_response(
    r: proto_prog::RecordActivityResponse,
) -> RecordActivityResponse {
    RecordActivityResponse {
        user: r
            .user
            .map(proto_user_level_to_response)
            .unwrap_or(UserLevelResponse {
                user_id: String::new(),
                username: String::new(),
                xp: 0,
                level: 0,
                xp_text: 0,
                level_text: 0,
                xp_text_current: 0,
                xp_text_needed: 0,
                xp_voice: 0,
                level_voice: 0,
                xp_voice_current: 0,
                xp_voice_needed: 0,
            }),
        leveled_up: r.leveled_up,
        old_level_global: r.old_level_global,
        xp_gained: r.xp_gained,
        skipped: r.skipped,
        streak_current: r.streak_current,
    }
}

fn proto_user_stats_to_response(u: proto_stats::UserStats) -> UserStatsResponse {
    UserStatsResponse {
        username: u.username,
        message_count: u.message_count,
        voice_seconds: u.voice_seconds,
    }
}

use crate::shared::grpc_client::grpc_err_to_string;
