//! Service wallet partage — socle commun de tous les jeux Nexus.
//!
//! Implemente `GetWalletUseCase`, `TransferCoinsUseCase`,
//! `GetWalletHistoryUseCase` et `GetWalletLeaderboardUseCase`. La creation
//! d'un wallet credite le solde de depart (`starting_coins` par guild,
//! defaut historique 100) et journalise une transaction `starting_coins`.

use std::sync::Arc;

use async_trait::async_trait;

use crate::nexus::domain::entities::wallet::clamp_limit;
use crate::nexus::domain::entities::wallet::resolve_starting_coins;
use crate::nexus::domain::entities::wallet::validate_transfer;
use crate::nexus::domain::entities::wallet::Wallet;
use crate::nexus::domain::entities::wallet::WalletMutation;
use crate::nexus::domain::entities::wallet::WalletTransaction;
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::inbound::get_wallet::GetWalletUseCase;
use crate::nexus::ports::inbound::transfer_coins::TransferCoinsCommand;
use crate::nexus::ports::inbound::transfer_coins::TransferCoinsResult;
use crate::nexus::ports::inbound::transfer_coins::TransferCoinsUseCase;
use crate::nexus::ports::inbound::wallet_history::GetWalletHistoryUseCase;
use crate::nexus::ports::inbound::wallet_leaderboard::GetWalletLeaderboardUseCase;
use crate::nexus::ports::outbound::wallet_repository::WalletRepository;

/// Bornes de pagination (defaut / max) de l'historique.
pub const HISTORY_DEFAULT_LIMIT: i64 = 10;
pub const HISTORY_MAX_LIMIT: i64 = 50;
/// Bornes du leaderboard (defaut historique : top 10 affiche par le bot).
pub const LEADERBOARD_DEFAULT_LIMIT: i64 = 10;
pub const LEADERBOARD_MAX_LIMIT: i64 = 50;

/// Charge le wallet, ou le cree avec le solde de depart de la guild
/// (transaction `starting_coins` journalisee si > 0). Point d'entree
/// commun a tous les jeux : la wheel passe aussi par la.
pub async fn get_or_create_wallet(
    repo: &dyn WalletRepository,
    guild_id: &str,
    user_id: &str,
    username: &str,
) -> Result<Wallet, DomainError> {
    if let Some(existing) = repo.find(guild_id, user_id).await? {
        return Ok(existing);
    }

    let mut wallet = Wallet::new(guild_id, user_id);
    wallet.username = username.to_string();
    let starting = resolve_starting_coins(repo.starting_coins(guild_id).await?);
    if starting > 0 {
        wallet.credit(starting)?;
        let mutation = WalletMutation {
            amount: starting,
            balance_after: wallet.coins,
            source: "starting_coins".into(),
            description: "Solde de depart".into(),
            reason: None,
        };
        repo.save_with_transaction(&wallet, &mutation).await?;
    }
    Ok(wallet)
}

pub struct WalletService {
    repo: Arc<dyn WalletRepository>,
    config_repo:
        Arc<dyn crate::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository>,
}

impl WalletService {
    pub fn new(
        repo: Arc<dyn WalletRepository>,
        config_repo: Arc<
            dyn crate::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository,
        >,
    ) -> Self {
        Self { repo, config_repo }
    }
}

#[async_trait]
impl GetWalletUseCase for WalletService {
    async fn get(&self, guild_id: &str, user_id: &str) -> Result<Wallet, DomainError> {
        get_or_create_wallet(self.repo.as_ref(), guild_id, user_id, "").await
    }
}

#[async_trait]
impl TransferCoinsUseCase for WalletService {
    async fn transfer(
        &self,
        cmd: TransferCoinsCommand,
    ) -> Result<TransferCoinsResult, DomainError> {
        // Les deux wallets existent (creation avec starting_coins si besoin).
        let from = get_or_create_wallet(
            self.repo.as_ref(),
            &cmd.guild_id,
            &cmd.from_user_id,
            &cmd.from_username,
        )
        .await?;
        get_or_create_wallet(
            self.repo.as_ref(),
            &cmd.guild_id,
            &cmd.to_user_id,
            &cmd.to_username,
        )
        .await?;

        // Regles pures : auto-transfert, montant, solde suffisant (refus
        // explicite — le repo re-verifie sous verrou dans la transaction).
        validate_transfer(&cmd.from_user_id, &cmd.to_user_id, cmd.amount, from.coins)?;

        // Bornes propres au serveur, par-dessus les regles universelles.
        let cfg = crate::nexus::application::economy_config::load_economy(
            &self.config_repo,
            &cmd.guild_id,
        )
        .await?;
        cfg.validate_transfer(cmd.amount)
            .map_err(DomainError::Validation)?;

        let outcome = self
            .repo
            .transfer_atomic(
                &cmd.guild_id,
                &cmd.from_user_id,
                &cmd.to_user_id,
                cmd.amount,
                cmd.reason.as_deref(),
            )
            .await?;

        Ok(TransferCoinsResult {
            amount: cmd.amount,
            from_balance: outcome.from_balance,
            to_balance: outcome.to_balance,
        })
    }
}

#[async_trait]
impl GetWalletHistoryUseCase for WalletService {
    async fn history(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<WalletTransaction>, DomainError> {
        let limit = clamp_limit(limit, HISTORY_DEFAULT_LIMIT, HISTORY_MAX_LIMIT);
        let offset = offset.unwrap_or(0).max(0);
        self.repo.history(guild_id, user_id, limit, offset).await
    }
}

#[async_trait]
impl GetWalletLeaderboardUseCase for WalletService {
    async fn leaderboard(
        &self,
        guild_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<Wallet>, DomainError> {
        let limit = clamp_limit(limit, LEADERBOARD_DEFAULT_LIMIT, LEADERBOARD_MAX_LIMIT);
        self.repo.leaderboard(guild_id, limit).await
    }
}

#[cfg(test)]
#[path = "tests/wallet_service.rs"]
mod tests;
