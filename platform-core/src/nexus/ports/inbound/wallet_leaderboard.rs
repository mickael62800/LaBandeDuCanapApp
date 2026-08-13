//! Port inbound : classement des wallets d'une guild.

use async_trait::async_trait;

use crate::nexus::domain::entities::wallet::Wallet;
use crate::nexus::domain::errors::DomainError;

#[async_trait]
pub trait GetWalletLeaderboardUseCase: Send + Sync {
    /// Top N wallets par solde decroissant. `limit` optionnel (defaut et
    /// borne appliques par le service).
    async fn leaderboard(
        &self,
        guild_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<Wallet>, DomainError>;
}
