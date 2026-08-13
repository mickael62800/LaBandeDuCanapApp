//! RCON client port — abstraction d'une connexion RCON (Source/Minecraft).

use async_trait::async_trait;

use crate::nexus::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct RconConnectionParams {
    pub host: String,
    pub port: u16,
    pub password: String,
    pub timeout_secs: u32,
}

/// Reponse RCON. Pour Minecraft : list players, MOTD changes, kicks, etc.
#[derive(Debug, Clone)]
pub struct RconResponse {
    pub raw: String,
}

#[async_trait]
pub trait RconClient: Send + Sync {
    /// Execute une commande RCON one-shot (connect + auth + cmd + close).
    /// Pas de connexion persistante : keep simple, pas de pool.
    async fn execute(
        &self,
        params: &RconConnectionParams,
        command: &str,
    ) -> Result<RconResponse, DomainError>;
}
