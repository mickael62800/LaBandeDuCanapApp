//! Phase 7A.opt F.3 — Community (sponsorships + temp-roles) gRPC.
//!
//! Adaptateur inbound MINCE : parse/mappe le proto et delegue toute la logique
//! metier + persistance au use case `ManageSponsorshipsUseCase` (aucun sqlx ni
//! pg_pool ici).

use std::sync::Arc;

use tonic::Request;
use tonic::Response;
use tonic::Status;

use platform_core::sentinel::ports::inbound::community::check_eligibility::{
    CheckEligibilityUseCase, CheckRoleEligibilityCommand, ValidateSponsorshipCommand,
};
use platform_core::sentinel::ports::inbound::community::manage_monthly_ranking::ManageMonthlyRankingUseCase;
use platform_core::sentinel::ports::inbound::community::manage_sponsorships::ManageSponsorshipsUseCase;
use platform_proto::sentinel::community::v1 as proto;
use platform_proto::sentinel::community::v1::community_service_server::CommunityService;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;

pub struct CommunityGrpc {
    pub uc: Arc<dyn ManageSponsorshipsUseCase>,
    /// Prerequis de role et regles de parrainage : la DECISION vit ici, pas
    /// dans le bot, qui ne fournit que les faits Discord.
    pub eligibility_uc: Arc<dyn CheckEligibilityUseCase>,
    pub monthly_ranking_uc: Arc<dyn ManageMonthlyRankingUseCase>,
}

#[tonic::async_trait]
impl CommunityService for CommunityGrpc {
    // ── Sponsorships ──

    async fn create_sponsorship(
        &self,
        request: Request<proto::CreateSponsorshipRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .create_sponsorship(&req.guild_id, &req.sponsor_id, &req.sponsored_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn list_sponsorships(
        &self,
        request: Request<proto::ListSponsorshipsRequest>,
    ) -> Result<Response<proto::SponsorshipList>, Status> {
        let req = request.into_inner();
        let rows = self
            .uc
            .list_sponsorships(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::SponsorshipList {
            sponsorships: rows
                .into_iter()
                .map(|r| proto::Sponsorship {
                    id: r.id.to_string(),
                    guild_id: r.guild_id.into_inner(),
                    sponsor_id: r.sponsor_id,
                    sponsored_id: r.sponsored_id,
                    created_at: r.created_at.to_rfc3339(),
                })
                .collect(),
        }))
    }

    // ── Temp Roles ──

    async fn create_temp_role(
        &self,
        request: Request<proto::CreateTempRoleRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .create_temp_role(&req.guild_id, &req.user_id, &req.role_id, &req.expires_at)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn list_temp_roles(
        &self,
        request: Request<proto::ListTempRolesRequest>,
    ) -> Result<Response<proto::TempRoleList>, Status> {
        let req = request.into_inner();
        let rows = self
            .uc
            .list_temp_roles(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::TempRoleList {
            roles: rows
                .into_iter()
                .map(|r| proto::TempRole {
                    id: r.id.to_string(),
                    guild_id: r.guild_id.into_inner(),
                    user_id: r.user_id.into_inner(),
                    role_id: r.role_id.into_inner(),
                    expires_at: r.expires_at.to_rfc3339(),
                    created_at: r.created_at.to_rfc3339(),
                })
                .collect(),
        }))
    }

    async fn delete_temp_role(
        &self,
        request: Request<proto::DeleteTempRoleRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .delete_temp_role(&req.guild_id, &req.user_id, &req.role_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    // ── Eligibilite ──

    async fn check_role_eligibility(
        &self,
        request: Request<proto::CheckRoleEligibilityRequest>,
    ) -> Result<Response<proto::EligibilityDecision>, Status> {
        let req = request.into_inner();
        if req.guild_id.is_empty() {
            return Err(Status::invalid_argument("guild_id requis"));
        }
        let decision = self
            .eligibility_uc
            .check_role_eligibility(CheckRoleEligibilityCommand {
                guild_id: req.guild_id,
                role_id: req.role_id,
                user_roles: req.user_roles,
                joined_at_unix: req.joined_at_unix,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::EligibilityDecision {
            allowed: decision.allowed,
            reason: decision.reason,
        }))
    }

    async fn validate_sponsorship_eligibility(
        &self,
        request: Request<proto::ValidateSponsorshipRequest>,
    ) -> Result<Response<proto::EligibilityDecision>, Status> {
        let req = request.into_inner();
        if req.guild_id.is_empty() {
            return Err(Status::invalid_argument("guild_id requis"));
        }
        let decision = self
            .eligibility_uc
            .validate_sponsorship(ValidateSponsorshipCommand {
                guild_id: req.guild_id,
                sponsor_id: req.sponsor_id,
                sponsored_id: req.sponsored_id,
                sponsor_joined_at_unix: req.sponsor_joined_at_unix,
                sponsored_joined_at_unix: req.sponsored_joined_at_unix,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::EligibilityDecision {
            allowed: decision.allowed,
            reason: decision.reason,
        }))
    }

    // ── Classement mensuel ──

    async fn force_monthly_ranking(
        &self,
        request: Request<proto::ForceMonthlyRankingRequest>,
    ) -> Result<Response<proto::MonthlyRanking>, Status> {
        let req = request.into_inner();
        if req.guild_id.is_empty() {
            return Err(Status::invalid_argument("guild_id requis"));
        }
        // Chaine vide = periode par defaut ("actuel") : le bot n'a pas a
        // connaitre le libelle par defaut du serveur.
        let mois = Some(req.mois).filter(|m| !m.is_empty());
        let data = self
            .monthly_ranking_uc
            .force_ranking(&req.guild_id, mois)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::MonthlyRanking {
            period_label: data.period_label,
            note: data.note,
            text: ranking_entries_to_proto(data.text),
            voice: ranking_entries_to_proto(data.voice),
            global: ranking_entries_to_proto(data.global),
        }))
    }
}

fn ranking_entries_to_proto(
    entries: Vec<
        platform_core::sentinel::domain::entities::community::monthly_ranking::RankingEntry,
    >,
) -> Vec<proto::RankingEntry> {
    entries
        .into_iter()
        .map(|e| proto::RankingEntry {
            user_id: e.user_id,
            xp: e.xp,
        })
        .collect()
}
