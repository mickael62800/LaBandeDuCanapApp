//! Use case dataset IA : bornage des filtres de listing et validation des ids
//! de suppression (UUID + plafond). Le SQL vit dans `DatasetRepository`, le
//! handler HTTP ne fait que RBAC + parse/map.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::ai::dataset::{DatasetPage, DatasetQuery};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::ai::manage_dataset::{
    BulkDeleteCommand, ListDatasetQuery, ManageDatasetUseCase,
};
use crate::sentinel::ports::outbound::ai::dataset_repository::{
    DatasetRepository, NewDatasetMessage,
};

/// Plafond dur d'ids par requete de suppression (anti-abus).
const MAX_BULK_DELETE_IDS: usize = 5000;

pub struct ManageDatasetService {
    repo: Arc<dyn DatasetRepository>,
}

impl ManageDatasetService {
    pub fn new(repo: Arc<dyn DatasetRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageDatasetUseCase for ManageDatasetService {
    async fn collect_message(&self, msg: NewDatasetMessage) -> Result<(), DomainError> {
        if msg.guild_id.trim().is_empty()
            || msg.user_id.trim().is_empty()
            || msg.content.trim().is_empty()
        {
            return Err(DomainError::ValidationError(
                "guild_id, user_id et content requis".into(),
            ));
        }
        self.repo.insert_message(&msg).await
    }

    async fn list_messages(&self, query: ListDatasetQuery) -> Result<DatasetPage, DomainError> {
        let bounded = DatasetQuery {
            guild_id: query.guild_id,
            channel_id: query.channel_id,
            from: query.from,
            to: query.to,
            min_length: i64::from(query.min_length.unwrap_or(1).max(0)),
            limit: query
                .limit
                .unwrap_or(200)
                .clamp(1, crate::sentinel::application::validation::BATCH_LIMIT_MAX),
            offset: query.offset.unwrap_or(0).max(0),
        };
        self.repo.list_messages(&bounded).await
    }

    async fn bulk_delete(&self, cmd: BulkDeleteCommand) -> Result<i64, DomainError> {
        if cmd.ids.is_empty() {
            return Ok(0);
        }
        if cmd.ids.len() > MAX_BULK_DELETE_IDS {
            return Err(DomainError::ValidationError(
                "Max 5000 IDs par requete".into(),
            ));
        }
        // Chaque id doit etre un UUID parsable (garde-fou metier avant le SQL).
        let uuids: Vec<Uuid> = cmd
            .ids
            .iter()
            .map(|s| Uuid::parse_str(s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DomainError::ValidationError(format!("uuid invalide: {e}")))?;

        self.repo.bulk_delete(&cmd.guild_id, &uuids).await
    }
}
