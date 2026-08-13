//! Port inbound : historique pagine des transactions d'un wallet.

use async_trait::async_trait;

use crate::nexus::domain::entities::wallet::WalletTransaction;
use crate::nexus::domain::errors::DomainError;

#[async_trait]
pub trait GetWalletHistoryUseCase: Send + Sync {
    /// Transactions du wallet, plus recentes d'abord. `limit`/`offset`
    /// optionnels (defauts et bornes appliques par le service).
    async fn history(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<WalletTransaction>, DomainError>;
}
