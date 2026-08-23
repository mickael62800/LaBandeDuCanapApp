//! Membre du mois.
//!
//! Le service tient la regle qui donne son sens a la section : la raison est
//! obligatoire. Sans elle, on affiche un nom sans dire pourquoi, et la
//! distinction ne recompense plus rien.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::spotlight::{
    is_valid_period, period_of, Spotlight, UpsertSpotlightCommand,
};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_spotlight::ManageSpotlightUseCase;
use crate::sentinel::ports::outbound::community::spotlight_repository::SpotlightRepository;

const MAX_REASON_CHARS: usize = 400;
const MAX_LIMIT: i64 = 60;

pub struct ManageSpotlightService {
    repo: Arc<dyn SpotlightRepository>,
}

impl ManageSpotlightService {
    pub fn new(repo: Arc<dyn SpotlightRepository>) -> Self {
        Self { repo }
    }

    fn sanitize(mut cmd: UpsertSpotlightCommand) -> Result<UpsertSpotlightCommand, DomainError> {
        if cmd.user_id.trim().is_empty() {
            return Err(DomainError::ValidationError("membre obligatoire".into()));
        }

        cmd.reason = cmd
            .reason
            .trim()
            .chars()
            .take(MAX_REASON_CHARS)
            .collect::<String>();
        if cmd.reason.is_empty() {
            return Err(DomainError::ValidationError(
                "explique pourquoi : c'est ce qui donne du sens a la distinction".into(),
            ));
        }

        cmd.username = cmd.username.trim().to_string();
        cmd.avatar = cmd
            .avatar
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty());

        // Periode absente = mois courant, le cas de loin le plus frequent.
        cmd.period = Some(match cmd.period.map(|p| p.trim().to_string()) {
            Some(p) if !p.is_empty() => {
                if !is_valid_period(&p) {
                    return Err(DomainError::ValidationError(
                        "periode attendue au format AAAA-MM".into(),
                    ));
                }
                p
            }
            _ => period_of(Utc::now()),
        });

        Ok(cmd)
    }
}

#[async_trait]
impl ManageSpotlightUseCase for ManageSpotlightService {
    async fn current(
        &self,
        guild_id: &str,
        period: Option<&str>,
    ) -> Result<Option<Spotlight>, DomainError> {
        match period {
            Some(p) => {
                if !is_valid_period(p) {
                    return Err(DomainError::ValidationError(
                        "periode attendue au format AAAA-MM".into(),
                    ));
                }
                self.repo.find_by_period(guild_id, p).await
            }
            // Sans periode demandee, on prend le plus recent plutot que le
            // mois courant : tant que le staff n'a designe personne pour ce
            // mois-ci, la section continue de montrer le precedent au lieu de
            // disparaitre.
            None => self.repo.find_latest(guild_id).await,
        }
    }

    async fn list(&self, guild_id: &str, limit: i64) -> Result<Vec<Spotlight>, DomainError> {
        self.repo.list(guild_id, limit.clamp(1, MAX_LIMIT)).await
    }

    async fn designate(&self, cmd: UpsertSpotlightCommand) -> Result<Spotlight, DomainError> {
        self.repo.upsert(&Self::sanitize(cmd)?).await
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        if self.repo.delete(id).await? {
            Ok(())
        } else {
            Err(DomainError::NotFound("designation introuvable".into()))
        }
    }
}
