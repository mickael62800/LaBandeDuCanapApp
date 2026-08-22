use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::system::ticket::Ticket;
use crate::sentinel::domain::entities::system::ticket::TicketDetail;
use crate::sentinel::domain::entities::system::ticket::TicketMessage;
use crate::sentinel::domain::enums::system::ticket_status::TicketStatus;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::system::manage_tickets::AssignTicketCommand;
use crate::sentinel::ports::inbound::system::manage_tickets::CreateTicketCommand;
use crate::sentinel::ports::inbound::system::manage_tickets::ManageTicketsUseCase;
use crate::sentinel::ports::inbound::system::manage_tickets::ReplyTicketCommand;
use crate::sentinel::ports::inbound::system::manage_tickets::UpdateTicketChannelCommand;
use tracing::warn;

use crate::sentinel::ports::outbound::system::cache::CachePort;
use crate::sentinel::ports::outbound::system::cache_helpers::cached_json;
use crate::sentinel::ports::outbound::system::ticket_repository::TicketRepository;
const TICKETS_LIST_TTL: u64 = 60; // 1 minute
const TICKET_DETAIL_TTL: u64 = 120; // 2 minutes

pub struct ManageTicketsService {
    ticket_repo: Arc<dyn TicketRepository>,
    cache: Arc<dyn CachePort>,
}

impl ManageTicketsService {
    pub fn new(ticket_repo: Arc<dyn TicketRepository>, cache: Arc<dyn CachePort>) -> Self {
        Self { ticket_repo, cache }
    }

    async fn invalidate_tickets_cache(&self) {
        if let Err(e) = self.cache.invalidate("tickets:all").await {
            warn!(error = %e, "Echec invalidation cache tickets:all");
        }
        if let Err(e) = self.cache.invalidate_pattern("ticket:*").await {
            warn!(error = %e, "Echec invalidation cache ticket:*");
        }
    }
}

#[async_trait]
impl ManageTicketsUseCase for ManageTicketsService {
    async fn list_tickets(
        &self,
        status: Option<String>,
        priority: Option<String>,
        search: Option<String>,
        author_id: Option<String>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Ticket>, DomainError> {
        let has_filters =
            status.is_some() || priority.is_some() || search.is_some() || author_id.is_some();

        // Cache-first uniquement si pas de filtres et premiere page
        if !has_filters && offset == 0 {
            if let Some(json) = self.cache.get_json("tickets:all").await? {
                if let Ok(tickets) = serde_json::from_str::<Vec<Ticket>>(&json) {
                    return Ok(tickets);
                }
            }
        }

        let tickets = self
            .ticket_repo
            .find_all(
                status.as_deref(),
                priority.as_deref(),
                search.as_deref(),
                author_id.as_deref(),
                limit,
                offset,
            )
            .await?;

        // Populate cache uniquement si pas de filtres
        if !has_filters {
            if let Ok(json) = serde_json::to_string(&tickets) {
                if let Err(e) = self
                    .cache
                    .set_json("tickets:all", &json, TICKETS_LIST_TTL)
                    .await
                {
                    warn!(error = %e, "Echec cache set tickets:all");
                }
            }
        }

        Ok(tickets)
    }

    async fn get_ticket_detail(&self, id: &str) -> Result<TicketDetail, DomainError> {
        let cache_key = format!("ticket:{id}");
        cached_json(&self.cache, &cache_key, TICKET_DETAIL_TTL, || async {
            let uuid = id
                .parse::<Uuid>()
                .map_err(|_| DomainError::ValidationError(format!("ID ticket invalide : {id}")))?;

            let ticket = self
                .ticket_repo
                .find_by_id(uuid)
                .await?
                .ok_or(DomainError::Internal(format!("Ticket introuvable : {id}")))?;

            let messages = self.ticket_repo.find_messages(uuid).await?;
            Ok(TicketDetail { ticket, messages })
        })
        .await
    }

    async fn create_ticket(&self, cmd: CreateTicketCommand) -> Result<Ticket, DomainError> {
        let now = chrono::Utc::now();
        let ticket = Ticket {
            id: Uuid::new_v4(),
            title: cmd.title,
            status: "open".to_string(),
            priority: cmd.priority,
            author_id: cmd.author_id,
            author_name: cmd.author_name,
            assigned_to: None,
            server: cmd.server,
            guild_id: cmd.guild_id,
            category: cmd.category,
            ticket_type: cmd.ticket_type,
            channel_id: cmd.channel_id,
            voice_channel_id: None,
            invited_user_id: None,
            created_at: now,
            updated_at: now,
            messages_count: 0,
        };

        self.ticket_repo.save(&ticket).await?;
        self.invalidate_tickets_cache().await;

        Ok(ticket)
    }

    async fn reply_ticket(&self, cmd: ReplyTicketCommand) -> Result<(), DomainError> {
        let ticket_id = cmd.ticket_id.parse::<Uuid>().map_err(|_| {
            DomainError::ValidationError(format!("ID ticket invalide : {}", cmd.ticket_id))
        })?;

        // Lit le statut courant : une reponse ne doit JAMAIS reouvrir un
        // ticket ferme (reopen silencieux). Le bot mirroir les messages tapes
        // dans le salon ; un message tardif sur un ticket ferme est rejete
        // (le bot ignore l'erreur Conflict).
        let ticket = self
            .ticket_repo
            .find_by_id(ticket_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Ticket {ticket_id}")))?;

        let current = TicketStatus::from_str(&ticket.status).unwrap_or(TicketStatus::Open);
        if current == TicketStatus::Closed {
            return Err(DomainError::Conflict(
                "ticket ferme : la reponse ne peut pas reouvrir le ticket".to_string(),
            ));
        }

        let message = TicketMessage {
            id: Uuid::new_v4(),
            ticket_id,
            author_name: cmd.author_name,
            author_role: cmd.author_role,
            content: cmd.content,
            created_at: chrono::Utc::now(),
        };

        self.ticket_repo.save_message(&message).await?;
        // Transition open/pending -> pending (autorisee tant que non ferme).
        if TicketStatus::can_transition(current, TicketStatus::Pending) {
            if let Err(e) = self.ticket_repo.update_status(ticket_id, "pending").await {
                warn!(error = %e, ticket_id = %ticket_id, "Echec update status ticket vers pending");
            }
        }
        self.invalidate_tickets_cache().await;

        Ok(())
    }

    async fn close_ticket(&self, id: &str) -> Result<bool, DomainError> {
        let uuid = id
            .parse::<Uuid>()
            .map_err(|_| DomainError::ValidationError(format!("ID ticket invalide : {id}")))?;

        // Garde atomique : ne transitionne que si pas deja ferme.
        let claimed = self.ticket_repo.close_if_open(uuid).await?;
        if claimed {
            self.invalidate_tickets_cache().await;
        }
        Ok(claimed)
    }

    async fn update_status(&self, id: &str, status: &str) -> Result<(), DomainError> {
        let uuid = id
            .parse::<Uuid>()
            .map_err(|_| DomainError::ValidationError(format!("ID ticket invalide : {id}")))?;

        let target = TicketStatus::from_str(status).ok_or_else(|| {
            DomainError::ValidationError(format!(
                "Statut invalide : {status} (valides : {})",
                TicketStatus::VALID_VALUES.join(", ")
            ))
        })?;

        // Valide la transition d'etat : empeche une reouverture illegale
        // (closed -> pending). closed -> open reste possible (reouverture
        // explicite).
        let ticket = self
            .ticket_repo
            .find_by_id(uuid)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Ticket {uuid}")))?;
        let current = TicketStatus::from_str(&ticket.status).unwrap_or(TicketStatus::Open);
        if !TicketStatus::can_transition(current, target) {
            return Err(DomainError::Conflict(format!(
                "Transition de statut interdite : {current} -> {target}"
            )));
        }

        self.ticket_repo.update_status(uuid, status).await?;
        self.invalidate_tickets_cache().await;

        Ok(())
    }

    async fn assign_ticket(&self, cmd: AssignTicketCommand) -> Result<(), DomainError> {
        let uuid = cmd.ticket_id.parse::<Uuid>().map_err(|_| {
            DomainError::ValidationError(format!("ID ticket invalide : {}", cmd.ticket_id))
        })?;

        self.ticket_repo
            .update_assignee(uuid, &cmd.assignee)
            .await?;
        self.invalidate_tickets_cache().await;

        Ok(())
    }

    async fn update_ticket_channel(
        &self,
        cmd: UpdateTicketChannelCommand,
    ) -> Result<(), DomainError> {
        let uuid = cmd.ticket_id.parse::<Uuid>().map_err(|_| {
            DomainError::ValidationError(format!("ID ticket invalide : {}", cmd.ticket_id))
        })?;

        if let Some(ref vc_id) = cmd.voice_channel_id {
            self.ticket_repo
                .update_voice_channel(uuid, Some(vc_id))
                .await?;
        }
        if let Some(ref inv_id) = cmd.invited_user_id {
            self.ticket_repo
                .update_invited_user(uuid, Some(inv_id))
                .await?;
        }
        self.invalidate_tickets_cache().await;

        Ok(())
    }

    async fn update_priority(&self, id: Uuid, priority: &str) -> Result<(), DomainError> {
        self.ticket_repo.update_priority(id, priority).await?;
        self.invalidate_tickets_cache().await;
        Ok(())
    }

    async fn update_sla(
        &self,
        id: Uuid,
        first_response_at: Option<&str>,
        resolved_at: Option<&str>,
        satisfaction_rating: Option<i32>,
    ) -> Result<(), DomainError> {
        self.ticket_repo
            .update_sla(id, first_response_at, resolved_at, satisfaction_rating)
            .await?;
        Ok(())
    }

    async fn bulk_delete_tickets(
        &self,
        author_id: Option<&str>,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<u64, DomainError> {
        let deleted = self.ticket_repo.bulk_delete(author_id, from, to).await?;
        if deleted > 0 {
            self.invalidate_tickets_cache().await;
        }
        Ok(deleted)
    }
}

