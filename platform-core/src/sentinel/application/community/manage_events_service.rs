//! Planning communautaire : evenements et campagnes de jeu.
//!
//! Le service porte les regles qui ne doivent dependre ni de la base ni du
//! transport : coherence de la plage, bornes de titre, et normalisation des
//! champs libres.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::event::{
    CommunityEvent, EventAnswer, UpsertEventCommand,
};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_events::{
    EventWithParticipants, ManageEventsUseCase,
};
use crate::sentinel::ports::outbound::community::event_repository::{EventRepository, EventWindow};

/// Plafond de duree : 2 ans. Au-dela, c'est une erreur de saisie (annee mal
/// tapee), pas une campagne — et ca polluerait toutes les vues du calendrier.
const MAX_DURATION_DAYS: i64 = 730;

pub struct ManageEventsService {
    repo: Arc<dyn EventRepository>,
}

impl ManageEventsService {
    pub fn new(repo: Arc<dyn EventRepository>) -> Self {
        Self { repo }
    }

    /// Valide et normalise une commande avant ecriture.
    fn sanitize(mut cmd: UpsertEventCommand) -> Result<UpsertEventCommand, DomainError> {
        cmd.title = cmd.title.trim().to_string();
        if cmd.title.is_empty() {
            return Err(DomainError::ValidationError("titre obligatoire".into()));
        }
        if cmd.title.chars().count() > 120 {
            return Err(DomainError::ValidationError(
                "titre limite a 120 caracteres".into(),
            ));
        }

        if cmd.ends_at < cmd.starts_at {
            return Err(DomainError::ValidationError(
                "la fin ne peut pas preceder le debut".into(),
            ));
        }
        if (cmd.ends_at - cmd.starts_at).num_days() > MAX_DURATION_DAYS {
            return Err(DomainError::ValidationError(
                "duree superieure a deux ans : verifie les dates".into(),
            ));
        }

        // Champs libres : vide equivaut a absent, pour ne pas stocker des
        // chaines vides que le front devrait ensuite distinguer de NULL.
        cmd.description = cmd
            .description
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());
        cmd.game = cmd
            .game
            .map(|g| g.trim().to_string())
            .filter(|g| !g.is_empty());
        cmd.color = cmd
            .color
            .map(|c| c.trim().trim_start_matches('#').to_lowercase())
            .filter(|c| c.len() == 6 && c.chars().all(|ch| ch.is_ascii_hexdigit()));

        Ok(cmd)
    }
}

#[async_trait]
impl ManageEventsUseCase for ManageEventsService {
    async fn list_window(
        &self,
        guild_id: &str,
        window: EventWindow,
        public_only: bool,
    ) -> Result<Vec<CommunityEvent>, DomainError> {
        self.repo
            .list_in_window(guild_id, window, public_only)
            .await
    }

    async fn get(&self, id: Uuid) -> Result<EventWithParticipants, DomainError> {
        let event = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("evenement introuvable".into()))?;
        let participants = self.repo.list_participants(id).await?;
        Ok(EventWithParticipants {
            event,
            participants,
        })
    }

    async fn create(&self, cmd: UpsertEventCommand) -> Result<CommunityEvent, DomainError> {
        self.repo.create(&Self::sanitize(cmd)?).await
    }

    async fn update(
        &self,
        id: Uuid,
        cmd: UpsertEventCommand,
    ) -> Result<CommunityEvent, DomainError> {
        self.repo
            .update(id, &Self::sanitize(cmd)?)
            .await?
            .ok_or_else(|| DomainError::NotFound("evenement introuvable".into()))
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        if self.repo.delete(id).await? {
            Ok(())
        } else {
            Err(DomainError::NotFound("evenement introuvable".into()))
        }
    }

    async fn join(
        &self,
        event_id: Uuid,
        user_id: &str,
        username: &str,
        answer: EventAnswer,
    ) -> Result<(), DomainError> {
        // Verifie l'existence : sans ca, la contrainte de cle etrangere
        // remonterait une erreur d'infrastructure illisible.
        if self.repo.find_by_id(event_id).await?.is_none() {
            return Err(DomainError::NotFound("evenement introuvable".into()));
        }
        self.repo
            .set_participation(event_id, user_id, username.trim(), answer)
            .await
    }

    async fn leave(&self, event_id: Uuid, user_id: &str) -> Result<(), DomainError> {
        self.repo.remove_participation(event_id, user_id).await?;
        Ok(())
    }
}
