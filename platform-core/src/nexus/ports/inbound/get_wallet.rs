//! Port inbound : consultation du wallet.

use async_trait::async_trait;

use crate::nexus::domain::entities::wallet::Wallet;
use crate::nexus::domain::errors::DomainError;

#[async_trait]
pub trait GetWalletUseCase: Send + Sync {
    /// Retourne le wallet (guild, user), vierge si inexistant.
    async fn get(&self, guild_id: &str, user_id: &str) -> Result<Wallet, DomainError>;
}
