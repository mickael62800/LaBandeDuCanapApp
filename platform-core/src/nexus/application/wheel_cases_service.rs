//! Edition des cases de la Roue du Destin.
//!
//! La regle qui structure tout : ABSENCE DE LIGNE = ROUE HISTORIQUE. Aucune
//! guilde n'est semee a la creation, et effacer ses cases suffit a revenir a
//! la roue d'origine. Sans cette regle il faudrait dix lignes par serveur des
//! l'installation, et « revenir a la roue de base » deviendrait une operation
//! a part entiere.

use std::sync::Arc;

use async_trait::async_trait;

use crate::nexus::domain::entities::wheel::{default_cases, validate_cases, WheelCaseData};
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::inbound::wheel_cases::{ManageWheelCasesUseCase, WheelCases};
use crate::nexus::ports::outbound::wheel_repository::WheelRepository;

pub struct WheelCasesService {
    repo: Arc<dyn WheelRepository>,
}

impl WheelCasesService {
    pub fn new(repo: Arc<dyn WheelRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageWheelCasesUseCase for WheelCasesService {
    async fn list(&self, guild_id: &str) -> Result<WheelCases, DomainError> {
        let cases = self.repo.list_cases(guild_id).await?;
        if cases.is_empty() {
            return Ok(WheelCases {
                cases: default_cases(),
                customized: false,
            });
        }
        Ok(WheelCases {
            cases,
            customized: true,
        })
    }

    async fn replace(
        &self,
        guild_id: &str,
        cases: Vec<WheelCaseData>,
    ) -> Result<WheelCases, DomainError> {
        // Liste vide = retour a la roue historique, pas une erreur.
        if cases.is_empty() {
            self.repo.replace_cases(guild_id, &[]).await?;
            return self.list(guild_id).await;
        }

        // Valide AVANT d'ecrire : une roue invalide en base ferait echouer
        // tous les tirages du serveur, longtemps apres la saisie fautive, et
        // le lien entre les deux serait impossible a faire.
        validate_cases(&cases).map_err(DomainError::Validation)?;

        self.repo.replace_cases(guild_id, &cases).await?;
        self.list(guild_id).await
    }
}

#[cfg(test)]
#[path = "tests/wheel_cases.rs"]
mod tests;
