//! Port outbound : persistance du wallet Nexus.

use async_trait::async_trait;

use crate::nexus::domain::entities::wallet::Wallet;
use crate::nexus::domain::entities::wallet::WalletMutation;
use crate::nexus::domain::entities::wallet::WalletTransaction;
use crate::nexus::domain::errors::DomainError;

/// Soldes resultants d'un transfert atomique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferOutcome {
    pub from_balance: i64,
    pub to_balance: i64,
}

#[async_trait]
pub trait WalletRepository: Send + Sync {
    /// Charge le wallet (guild, user) s'il existe.
    async fn find(&self, guild_id: &str, user_id: &str) -> Result<Option<Wallet>, DomainError>;

    /// Solde de depart configure pour la guild (None = pas d'override,
    /// le domaine applique `DEFAULT_STARTING_COINS`).
    async fn starting_coins(&self, guild_id: &str) -> Result<Option<i64>, DomainError>;

    /// Persiste l'etat du wallet ET journalise la mutation dans
    /// `nexus_wallet_transactions` (upsert du wallet + insert transaction),
    /// le tout en une transaction DB.
    async fn save_with_transaction(
        &self,
        wallet: &Wallet,
        mutation: &WalletMutation,
    ) -> Result<(), DomainError>;

    /// Transfert atomique entre deux wallets d'une meme guild : debit exact
    /// de l'emetteur (REFUS si solde insuffisant, jamais de clamp) + credit
    /// du destinataire + journalisation `transfer_out`/`transfer_in`, en une
    /// seule transaction DB avec verrouillage ordonne des deux lignes.
    /// Les deux wallets doivent exister (le service les cree avant).
    async fn transfer_atomic(
        &self,
        guild_id: &str,
        from_user_id: &str,
        to_user_id: &str,
        amount: i64,
        reason: Option<&str>,
    ) -> Result<TransferOutcome, DomainError>;

    /// Historique pagine des transactions d'un wallet (plus recentes
    /// d'abord).
    async fn history(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WalletTransaction>, DomainError>;

    /// Top N wallets d'une guild par solde decroissant.
    async fn leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<Wallet>, DomainError>;
}
