//! Demarrage du serveur tonic en parallele d'Axum.
//!
//! Phase 7A optimisations :
//! - **Compression Gzip** activee en envoi et reception sur les 12 services
//!   (gain ~40-70% bande passante sur images, ~30% sur leaderboards, CPU <5%).
//! - **tonic-health** expose un service `grpc.health.v1.Health` qui permet au
//!   healthcheck Docker de verifier chaque service individuellement via
//!   `grpc_health_probe -addr=:50051`.
//!
//! Auth : si `api_key` est non vide, un interceptor verifie l'en-tete de
//! metadata `authorization: Bearer <api_key>` sur chaque appel. Sinon le
//! serveur tourne ouvert (mode dev) — meme logique qu'`adapters/inbound/http`.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use platform_proto::sentinel::age_gate::v1::age_gate_service_server::AgeGateServiceServer;
use platform_proto::sentinel::ai_dataset::v1::ai_dataset_service_server::AiDatasetServiceServer;
use platform_proto::sentinel::announcements::v1::announcements_service_server::AnnouncementsServiceServer;
use platform_proto::sentinel::audit::v1::audit_service_server::AuditServiceServer;
use platform_proto::sentinel::automod::v1::automod_service_server::AutomodServiceServer;
use platform_proto::sentinel::automod_review::v1::automod_review_service_server::AutomodReviewServiceServer;
use platform_proto::sentinel::community::v1::community_service_server::CommunityServiceServer;
use platform_proto::sentinel::confessions::v1::confessions_service_server::ConfessionsServiceServer;
use platform_proto::sentinel::discord_messages::v1::discord_action_messages_service_server::DiscordActionMessagesServiceServer;
use platform_proto::sentinel::embeds::v1::embeds_service_server::EmbedsServiceServer;
use platform_proto::sentinel::export::v1::export_service_server::ExportServiceServer;
use platform_proto::sentinel::guild_backup::v1::guild_backup_service_server::GuildBackupServiceServer;
use platform_proto::sentinel::ideas::v1::ideas_service_server::IdeasServiceServer;
use platform_proto::sentinel::images::v1::images_service_server::ImagesServiceServer;
use platform_proto::sentinel::moderation::v1::moderation_service_server::ModerationServiceServer;
use platform_proto::sentinel::progression::v1::progression_service_server::ProgressionServiceServer;
use platform_proto::sentinel::purge::v1::purge_service_server::PurgeServiceServer;
use platform_proto::sentinel::security::v1::security_service_server::SecurityServiceServer;
use platform_proto::sentinel::security_state::v1::security_state_service_server::SecurityStateServiceServer;
use platform_proto::sentinel::stats::v1::stats_service_server::StatsServiceServer;
use platform_proto::sentinel::sursis::v1::sursis_service_server::SursisServiceServer;
use platform_proto::sentinel::tickets::v1::tickets_service_server::TicketsServiceServer;
use platform_proto::sentinel::welcome::v1::welcome_service_server::WelcomeServiceServer;
use tonic::codec::CompressionEncoding;
use tonic::metadata::MetadataValue;
use tonic::service::interceptor::InterceptedService;
use tonic::transport::Server;
use tonic::Request;
use tonic::Status;
use tracing::info;

use crate::sentinel::adapters::inbound::grpc::ai::automod::AutomodGrpc;
use crate::sentinel::adapters::inbound::grpc::ai::dataset::AiDatasetGrpc;
use crate::sentinel::adapters::inbound::grpc::ai::images::ImagesGrpc;
use crate::sentinel::adapters::inbound::grpc::audit::action_messages::DiscordActionMessagesGrpc;
use crate::sentinel::adapters::inbound::grpc::audit::journal::AuditGrpc;
use crate::sentinel::adapters::inbound::grpc::audit::security::SecurityGrpc;
use crate::sentinel::adapters::inbound::grpc::audit::stats::StatsGrpc;
use crate::sentinel::adapters::inbound::grpc::community::age_gate::AgeGateGrpc;
use crate::sentinel::adapters::inbound::grpc::community::announcements::AnnouncementsGrpc;
use crate::sentinel::adapters::inbound::grpc::community::confessions::ConfessionsGrpc;
use crate::sentinel::adapters::inbound::grpc::community::embeds::EmbedsGrpc;
use crate::sentinel::adapters::inbound::grpc::community::ideas::IdeasGrpc;
use crate::sentinel::adapters::inbound::grpc::community::progression::ProgressionGrpc;
use crate::sentinel::adapters::inbound::grpc::community::sponsorships::CommunityGrpc;
use crate::sentinel::adapters::inbound::grpc::guild_backup::snapshots::GuildBackupGrpc;
use crate::sentinel::adapters::inbound::grpc::moderation::actions::ModerationGrpc;
use crate::sentinel::adapters::inbound::grpc::moderation::purge::PurgeGrpc;
use crate::sentinel::adapters::inbound::grpc::moderation::reviews::AutomodReviewGrpc;
use crate::sentinel::adapters::inbound::grpc::moderation::sursis::SursisGrpc;
use crate::sentinel::adapters::inbound::grpc::system::export::ExportGrpc;
use crate::sentinel::adapters::inbound::grpc::system::security_state::SecurityStateGrpc;
use crate::sentinel::adapters::inbound::grpc::system::tickets::TicketsGrpc;
use crate::sentinel::adapters::inbound::grpc::system::welcome::WelcomeGrpc;
use crate::sentinel::adapters::inbound::http::state::AppState;

/// Lance le serveur gRPC. A spawn dans une task tokio depuis `main.rs`.
pub async fn serve_grpc(state: AppState, bind: SocketAddr) -> Result<(), String> {
    let api_key = state.shared.api_key.clone();

    let progression = ProgressionGrpc {
        levels_uc: state.community.levels_uc.clone(),
        broadcaster: state.shared.broadcaster.clone(),
    };
    let stats = StatsGrpc {
        stats_uc: state.audit.stats_uc.clone(),
        broadcaster: state.shared.broadcaster.clone(),
    };
    let tickets = TicketsGrpc {
        tickets_uc: state.system.tickets_uc.clone(),
    };
    let moderation = ModerationGrpc {
        moderation_uc: state.moderation.moderation_uc.clone(),
        cancel_action_uc: state.moderation.cancel_action_uc.clone(),
        assess_target_risk_uc: state.moderation.assess_target_risk_uc.clone(),
        modstats_uc: state.moderation.modstats_uc.clone(),
        evidence_repo: state.moderation.evidence_repo.clone(),
        review_repo: state.moderation.review_repo.clone(),
        pending_action_repo: state.moderation.pending_action_repo.clone(),
        infractions_uc: state.moderation.infractions_uc.clone(),
        manage_reminders_uc: state.moderation.manage_reminders_uc.clone(),
    };
    let welcome = WelcomeGrpc {
        uc: state.community.welcome_config_uc.clone(),
    };
    let audit = AuditGrpc {
        audit_logs_uc: state.audit.audit_logs_uc.clone(),
        watched_users_uc: state.audit.watched_users_uc.clone(),
        weekly_report_uc: state.audit.weekly_report_uc.clone(),
        detect_anomaly_uc: state.audit.detect_anomaly_uc.clone(),
        user_activity_repo: state.audit.user_activity_repo.clone(),
    };
    let guild_backup = GuildBackupGrpc {
        snapshots_uc: state.guild_backup.guild_snapshots_uc.clone(),
        pending_role_grants_uc: state.guild_backup.pending_role_grants_uc.clone(),
    };
    let ideas = IdeasGrpc {
        uc: state.community.ideas_uc.clone(),
    };
    let purge = PurgeGrpc {
        infractions_uc: state.moderation.infractions_uc.clone(),
        audit_logs_uc: state.audit.audit_logs_uc.clone(),
        log_repo: state.shared.log_repo.clone(),
        broadcaster: state.shared.broadcaster.clone(),
    };
    let export = ExportGrpc {
        uc: state.system.export_uc.clone(),
    };
    let community = CommunityGrpc {
        uc: state.community.manage_sponsorships_uc.clone(),
        eligibility_uc: state.community.eligibility_uc.clone(),
        monthly_ranking_uc: state.community.monthly_ranking_uc.clone(),
    };
    let security = SecurityGrpc {
        uc: state.audit.security_uc.clone(),
    };
    let security_state = SecurityStateGrpc {
        quarantine_uc: state.system.quarantine_uc.clone(),
        slowmode_uc: state.system.slowmode_uc.clone(),
        lockdown_uc: state.system.lockdown_uc.clone(),
    };
    let automod_review = AutomodReviewGrpc {
        reviews_uc: state.moderation.automod_reviews_uc.clone(),
        moderation_uc: state.moderation.moderation_uc.clone(),
        bot_config_repo: state.moderation.bot_config_repo.clone(),
        broadcaster: state.shared.broadcaster.clone(),
    };
    let action_messages = DiscordActionMessagesGrpc {
        uc: state.audit.discord_action_messages_uc.clone(),
    };
    let sursis = SursisGrpc {
        sursis_uc: state.moderation.sursis_uc.clone(),
        bot_config_repo: state.moderation.bot_config_repo.clone(),
    };
    let confessions = ConfessionsGrpc {
        uc: state.community.confessions_uc.clone(),
    };
    let announcements = AnnouncementsGrpc {
        uc: state.community.announcements_uc.clone(),
    };
    let age_gate = AgeGateGrpc {
        age_check_uc: state.community.age_check_uc.clone(),
        age_ban_repo: state.community.age_ban_repo.clone(),
    };
    let embeds = EmbedsGrpc {
        uc: state.community.embeds_uc.clone(),
    };
    let automod = AutomodGrpc {
        uc: state.ai.analyze_uc.clone(),
        broadcaster: state.shared.broadcaster.clone(),
        adaptive_slowmode_repo: state.moderation.automod_adaptive_slowmode_repo.clone(),
    };
    let images = ImagesGrpc {
        uc: state.ai.analyze_image_uc.clone(),
    };
    let ai_dataset = AiDatasetGrpc {
        dataset_uc: state.ai.dataset_uc.clone(),
    };

    let auth_interceptor = build_auth_interceptor(api_key)?;

    // Helper local : compression Gzip (send/accept) puis wrap dans l'auth
    // interceptor. Les methodes `send_compressed`/`accept_compressed` sont sur
    // le ServiceServer ; l'interceptor vient par-dessus via InterceptedService.
    macro_rules! svc {
        ($ServerType:ident, $impl:expr) => {{
            let inner = $ServerType::new($impl)
                .send_compressed(CompressionEncoding::Gzip)
                .accept_compressed(CompressionEncoding::Gzip);
            InterceptedService::new(inner, auth_interceptor.clone())
        }};
    }
    let progression_svc = svc!(ProgressionServiceServer, progression);
    let stats_svc = svc!(StatsServiceServer, stats);
    let tickets_svc = svc!(TicketsServiceServer, tickets);
    let moderation_svc = svc!(ModerationServiceServer, moderation);
    let security_svc = svc!(SecurityServiceServer, security);
    let security_state_svc = svc!(SecurityStateServiceServer, security_state);
    let automod_review_svc = svc!(AutomodReviewServiceServer, automod_review);
    let action_messages_svc = svc!(DiscordActionMessagesServiceServer, action_messages);
    let sursis_svc = svc!(SursisServiceServer, sursis);
    let confessions_svc = svc!(ConfessionsServiceServer, confessions);
    let announcements_svc = svc!(AnnouncementsServiceServer, announcements);
    let age_gate_svc = svc!(AgeGateServiceServer, age_gate);
    let embeds_svc = svc!(EmbedsServiceServer, embeds);
    let automod_svc = svc!(AutomodServiceServer, automod);
    let images_svc = svc!(ImagesServiceServer, images);
    // Phase 7A.opt F.3/F.4 — nouveaux services.
    let welcome_svc = svc!(WelcomeServiceServer, welcome);
    let audit_svc = svc!(AuditServiceServer, audit);
    let guild_backup_svc = svc!(GuildBackupServiceServer, guild_backup);
    let ideas_svc = svc!(IdeasServiceServer, ideas);
    let purge_svc = svc!(PurgeServiceServer, purge);
    let export_svc = svc!(ExportServiceServer, export);
    let community_svc = svc!(CommunityServiceServer, community);
    let ai_dataset_svc = svc!(AiDatasetServiceServer, ai_dataset);

    // tonic-health : expose `grpc.health.v1.Health` + marque chaque service
    // comme SERVING. Permet `grpc_health_probe -addr=:50051` dans le healthcheck.
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<ProgressionServiceServer<ProgressionGrpc>>()
        .await;
    health_reporter
        .set_serving::<StatsServiceServer<StatsGrpc>>()
        .await;
    health_reporter
        .set_serving::<TicketsServiceServer<TicketsGrpc>>()
        .await;
    health_reporter
        .set_serving::<ModerationServiceServer<ModerationGrpc>>()
        .await;
    health_reporter
        .set_serving::<SecurityServiceServer<SecurityGrpc>>()
        .await;
    health_reporter
        .set_serving::<SecurityStateServiceServer<SecurityStateGrpc>>()
        .await;
    health_reporter
        .set_serving::<AutomodReviewServiceServer<AutomodReviewGrpc>>()
        .await;
    health_reporter
        .set_serving::<DiscordActionMessagesServiceServer<DiscordActionMessagesGrpc>>()
        .await;
    health_reporter
        .set_serving::<SursisServiceServer<SursisGrpc>>()
        .await;
    health_reporter
        .set_serving::<ConfessionsServiceServer<ConfessionsGrpc>>()
        .await;
    health_reporter
        .set_serving::<AnnouncementsServiceServer<AnnouncementsGrpc>>()
        .await;
    health_reporter
        .set_serving::<AgeGateServiceServer<AgeGateGrpc>>()
        .await;
    health_reporter
        .set_serving::<EmbedsServiceServer<EmbedsGrpc>>()
        .await;
    health_reporter
        .set_serving::<AutomodServiceServer<AutomodGrpc>>()
        .await;
    health_reporter
        .set_serving::<ImagesServiceServer<ImagesGrpc>>()
        .await;
    health_reporter
        .set_serving::<WelcomeServiceServer<WelcomeGrpc>>()
        .await;
    health_reporter
        .set_serving::<AuditServiceServer<AuditGrpc>>()
        .await;
    health_reporter
        .set_serving::<GuildBackupServiceServer<GuildBackupGrpc>>()
        .await;
    health_reporter
        .set_serving::<IdeasServiceServer<IdeasGrpc>>()
        .await;
    health_reporter
        .set_serving::<PurgeServiceServer<PurgeGrpc>>()
        .await;
    health_reporter
        .set_serving::<ExportServiceServer<ExportGrpc>>()
        .await;
    health_reporter
        .set_serving::<CommunityServiceServer<CommunityGrpc>>()
        .await;
    health_reporter
        .set_serving::<AiDatasetServiceServer<AiDatasetGrpc>>()
        .await;

    // HTTP/2 clair est permis uniquement lorsque TLS n'est pas configure.
    // Une configuration mTLS declaree mais invalide doit arreter le processus :
    // continuer en clair exposerait silencieusement les appels inter-services.
    let tls_dir = platform_proto::sentinel::tls::tls_dir();
    let mut server_builder = build_server_builder(tls_dir.as_deref())?;
    if let Some(dir) = tls_dir {
        info!(dir = %dir.display(), "gRPC mTLS active (server + client cert verification)");
    } else {
        info!("gRPC plain HTTP/2 (GRPC_TLS_DIR non defini)");
    }

    info!(addr = %bind, "Sentinel gRPC pret (compression Gzip + health)");

    server_builder
        .add_service(health_service)
        .add_service(progression_svc)
        .add_service(stats_svc)
        .add_service(tickets_svc)
        .add_service(moderation_svc)
        .add_service(security_svc)
        .add_service(security_state_svc)
        .add_service(automod_review_svc)
        .add_service(action_messages_svc)
        .add_service(sursis_svc)
        .add_service(confessions_svc)
        .add_service(announcements_svc)
        .add_service(age_gate_svc)
        .add_service(embeds_svc)
        .add_service(automod_svc)
        .add_service(images_svc)
        .add_service(welcome_svc)
        .add_service(export_svc)
        .add_service(purge_svc)
        .add_service(ideas_svc)
        .add_service(guild_backup_svc)
        .add_service(audit_svc)
        .add_service(community_svc)
        .add_service(ai_dataset_svc)
        .serve(bind)
        .await
        .map_err(|e| format!("Erreur serveur gRPC: {e}"))
}

fn build_server_builder(tls_dir: Option<&Path>) -> Result<Server, String> {
    let Some(dir) = tls_dir else {
        return Ok(Server::builder());
    };

    let config = platform_proto::sentinel::tls::server_tls_config(dir).map_err(|e| {
        format!(
            "impossible de lire la configuration mTLS dans {}: {e}",
            dir.display()
        )
    })?;

    Server::builder()
        .tls_config(config)
        .map_err(|e| format!("configuration mTLS invalide dans {}: {e}", dir.display()))
}

fn build_auth_interceptor(
    api_key: String,
) -> Result<impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone, String> {
    let expected: Option<Arc<MetadataValue<tonic::metadata::Ascii>>> = if api_key.is_empty() {
        None
    } else {
        match format!("Bearer {api_key}").parse::<MetadataValue<_>>() {
            Ok(v) => Some(Arc::new(v)),
            // Fail-CLOSED. Retomber sur `None` ouvrait les 23 services gRPC
            // (moderation, purge, export, confessions, guild_backup...) a qui
            // pouvait joindre le port, avec pour seule trace une ligne d'erreur
            // au demarrage : le chemin HTTP, lui, compare des octets bruts et
            // continuait de fonctionner, donc rien ne paraissait casse.
            // Une cle non-ASCII est une erreur de configuration, au meme titre
            // qu'une cle absente ou trop courte — que `config.rs` traite deja
            // par un arret immediat.
            Err(_) => {
                return Err(
                    "SENTINEL_API_KEY contient des caracteres invalides pour un en-tete gRPC \
                 (ASCII imprimable uniquement, ni retour a la ligne ni accent)"
                        .into(),
                )
            }
        }
    };

    Ok(move |req: Request<()>| {
        let Some(expected) = expected.as_ref() else {
            return Ok(req);
        };
        match req.metadata().get("authorization") {
            // Comparaison constant-time (comme le chemin HTTP auth.rs) pour
            // eviter une timing attack sur l'API key via gRPC.
            Some(token)
                if bool::from(subtle::ConstantTimeEq::ct_eq(
                    token.as_bytes(),
                    expected.as_bytes(),
                )) =>
            {
                Ok(req)
            }
            _ => Err(Status::unauthenticated("API key invalide ou manquante")),
        }
    })
}

#[cfg(test)]
#[path = "tests/server.rs"]
mod tests;
