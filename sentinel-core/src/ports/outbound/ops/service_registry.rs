//! Port de découverte des services (bots/workers) actifs.
//! L'adapter (Redis) lit `bots:known` + ping `bot:online:<name>`.

use async_trait::async_trait;

#[derive(Debug, Clone, Copy)]
pub struct ServiceCounts {
    pub bots_online: u32,
    pub bots_total: u32,
    pub workers_online: u32,
    pub workers_total: u32,
}

#[async_trait]
pub trait ServiceRegistry: Send + Sync {
    /// Retourne (bots_online, bots_total, workers_online, workers_total).
    /// Les erreurs infrastructure sont absorbees a l'interieur (retourne 0 partout).
    async fn count_services(&self) -> ServiceCounts;

    /// Health check du backend (typiquement : Redis PING). Retourne false
    /// si l'infra est down ou plante.
    async fn ping(&self) -> bool;
}
