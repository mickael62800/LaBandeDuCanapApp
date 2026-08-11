//! Use case Game Servers — interface utilise'e par les handlers HTTP et le worker.

use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::entities::game::server::{CreateGameServerCommand, GameServer};
use crate::domain::errors::DomainError;
use crate::ports::outbound::game::container_runtime::ContainerStats;

#[derive(Debug, Clone)]
pub struct GameServerDetail {
    pub server: GameServer,
    pub config: HashMap<String, String>,
}

#[async_trait]
pub trait ManageGameServersUseCase: Send + Sync {
    // ── CRUD basique ──────────────────────────────────────────────────
    async fn create(&self, cmd: CreateGameServerCommand) -> Result<GameServer, DomainError>;
    async fn list_for_guild(&self, guild_id: &str) -> Result<Vec<GameServer>, DomainError>;
    async fn get(&self, id: Uuid) -> Result<GameServerDetail, DomainError>;

    /// Soft-delete : stop container + remove + remove volume + soft-delete DB.
    async fn delete(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError>;

    // ── Lifecycle Docker ──────────────────────────────────────────────
    async fn start(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError>;
    async fn stop(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError>;
    async fn restart(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError>;

    /// Rend immédiatement l'adresse publique, avant l'échéance programmée.
    /// Réservé aux appels d'administration par la couche HTTP.
    async fn reveal_ip(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError>;

    // ── Observabilite ─────────────────────────────────────────────────
    async fn get_logs(&self, id: Uuid, lines: u32) -> Result<Vec<String>, DomainError>;
    async fn get_stats(&self, id: Uuid) -> Result<ContainerStats, DomainError>;

    // ── Config ────────────────────────────────────────────────────────
    /// Update full set d'overrides — atomique. Owner du serveur ou Admin+.
    async fn update_config(
        &self,
        id: Uuid,
        entries: HashMap<String, String>,
        actor_user_id: &str,
    ) -> Result<(), DomainError>;

    // ── Console RCON ──────────────────────────────────────────────────
    /// Execute une commande RCON (Owner uniquement). Retourne la reponse brute.
    async fn execute_rcon(
        &self,
        id: Uuid,
        command: &str,
        actor_user_id: &str,
    ) -> Result<String, DomainError>;
}
