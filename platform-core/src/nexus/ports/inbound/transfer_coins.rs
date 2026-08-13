//! Port inbound : transfert de coins entre joueurs (`/donner`).

use async_trait::async_trait;

use crate::nexus::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct TransferCoinsCommand {
    pub guild_id: String,
    pub from_user_id: String,
    pub from_username: String,
    pub to_user_id: String,
    pub to_username: String,
    pub amount: i64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferCoinsResult {
    pub amount: i64,
    pub from_balance: i64,
    pub to_balance: i64,
}

#[async_trait]
pub trait TransferCoinsUseCase: Send + Sync {
    /// Transfere `amount` coins entre deux joueurs de la guild.
    /// Regles pures : pas d'auto-transfert, montant > 0 borne par
    /// `MAX_WALLET_AMOUNT`, solde suffisant (refus explicite, pas de clamp).
    async fn transfer(&self, cmd: TransferCoinsCommand)
        -> Result<TransferCoinsResult, DomainError>;
}
