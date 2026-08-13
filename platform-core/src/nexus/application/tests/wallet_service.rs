//! Tests du WalletService sur mocks (pas de DB).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::nexus::application::wallet_service::WalletService;
use crate::nexus::domain::entities::wallet::Wallet;
use crate::nexus::domain::entities::wallet::WalletMutation;
use crate::nexus::domain::entities::wallet::WalletTransaction;
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::inbound::get_wallet::GetWalletUseCase;
use crate::nexus::ports::inbound::transfer_coins::TransferCoinsCommand;
use crate::nexus::ports::inbound::transfer_coins::TransferCoinsUseCase;
use crate::nexus::ports::inbound::wallet_history::GetWalletHistoryUseCase;
use crate::nexus::ports::inbound::wallet_leaderboard::GetWalletLeaderboardUseCase;
use crate::nexus::ports::outbound::wallet_repository::TransferOutcome;
use crate::nexus::ports::outbound::wallet_repository::WalletRepository;

// ── Mock ──

#[derive(Default)]
struct MockRepo {
    /// Wallets existants, cle "user_id".
    wallets: Mutex<HashMap<String, Wallet>>,
    /// Override starting_coins de la guild.
    guild_starting: Option<i64>,
    saved: Mutex<Vec<(Wallet, WalletMutation)>>,
    transfers: Mutex<Vec<(String, String, String, i64, Option<String>)>>,
    history_calls: Mutex<Vec<(i64, i64)>>,
    leaderboard_calls: Mutex<Vec<i64>>,
}

#[async_trait]
impl WalletRepository for MockRepo {
    async fn find(&self, _g: &str, u: &str) -> Result<Option<Wallet>, DomainError> {
        Ok(self.wallets.lock().unwrap().get(u).cloned())
    }
    async fn starting_coins(&self, _g: &str) -> Result<Option<i64>, DomainError> {
        Ok(self.guild_starting)
    }
    async fn save_with_transaction(
        &self,
        wallet: &Wallet,
        mutation: &WalletMutation,
    ) -> Result<(), DomainError> {
        self.wallets
            .lock()
            .unwrap()
            .insert(wallet.user_id.clone(), wallet.clone());
        self.saved
            .lock()
            .unwrap()
            .push((wallet.clone(), mutation.clone()));
        Ok(())
    }
    async fn transfer_atomic(
        &self,
        g: &str,
        f: &str,
        t: &str,
        amount: i64,
        reason: Option<&str>,
    ) -> Result<TransferOutcome, DomainError> {
        self.transfers.lock().unwrap().push((
            g.to_string(),
            f.to_string(),
            t.to_string(),
            amount,
            reason.map(str::to_string),
        ));
        let mut wallets = self.wallets.lock().unwrap();
        let from_balance = {
            let w = wallets.get_mut(f).expect("wallet emetteur cree avant");
            w.debit_exact(amount)?;
            w.coins
        };
        let to_balance = {
            let w = wallets.get_mut(t).expect("wallet destinataire cree avant");
            w.credit(amount)?;
            w.coins
        };
        Ok(TransferOutcome {
            from_balance,
            to_balance,
        })
    }
    async fn history(
        &self,
        _g: &str,
        _u: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WalletTransaction>, DomainError> {
        self.history_calls.lock().unwrap().push((limit, offset));
        Ok(vec![])
    }
    async fn leaderboard(&self, _g: &str, limit: i64) -> Result<Vec<Wallet>, DomainError> {
        self.leaderboard_calls.lock().unwrap().push(limit);
        Ok(vec![])
    }
}

fn service(repo: Arc<MockRepo>) -> WalletService {
    WalletService::new(
        repo,
        Arc::new(crate::nexus::application::economy_config::EmptyBotConfigRepository),
    )
}

fn seed_wallet(repo: &MockRepo, user: &str, coins: i64) {
    let mut w = Wallet::new("g1", user);
    if coins > 0 {
        w.credit(coins).unwrap();
    }
    repo.wallets.lock().unwrap().insert(user.to_string(), w);
}

fn transfer_cmd(from: &str, to: &str, amount: i64, reason: Option<&str>) -> TransferCoinsCommand {
    TransferCoinsCommand {
        guild_id: "g1".into(),
        from_user_id: from.into(),
        from_username: format!("name-{from}"),
        to_user_id: to.into(),
        to_username: format!("name-{to}"),
        amount,
        reason: reason.map(str::to_string),
    }
}

// ── get / get_or_create ──

#[tokio::test]
async fn get_creates_wallet_with_default_starting_coins() {
    let repo = Arc::new(MockRepo::default());
    let w = service(repo.clone()).get("g1", "u1").await.unwrap();
    assert_eq!(w.coins, 100, "defaut historique starting_coins = 100");
    let saved = repo.saved.lock().unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].1.source, "starting_coins");
    assert_eq!(saved[0].1.amount, 100);
    assert_eq!(saved[0].1.balance_after, 100);
}

#[tokio::test]
async fn get_uses_guild_starting_coins_override() {
    let repo = Arc::new(MockRepo {
        guild_starting: Some(500),
        ..Default::default()
    });
    let w = service(repo.clone()).get("g1", "u1").await.unwrap();
    assert_eq!(w.coins, 500);
    assert_eq!(repo.saved.lock().unwrap()[0].1.amount, 500);
}

#[tokio::test]
async fn get_with_zero_starting_coins_creates_nothing_persisted() {
    let repo = Arc::new(MockRepo {
        guild_starting: Some(0),
        ..Default::default()
    });
    let w = service(repo.clone()).get("g1", "u1").await.unwrap();
    assert_eq!(w.coins, 0);
    assert!(repo.saved.lock().unwrap().is_empty(), "pas de tx a 0");
}

#[tokio::test]
async fn get_returns_existing_wallet_without_recrediting() {
    let repo = Arc::new(MockRepo::default());
    seed_wallet(&repo, "u1", 42);
    let w = service(repo.clone()).get("g1", "u1").await.unwrap();
    assert_eq!(w.coins, 42, "wallet existant : pas de re-credit de depart");
    assert!(repo.saved.lock().unwrap().is_empty());
}

// ── transfer ──

#[tokio::test]
async fn transfer_moves_coins_between_players() {
    let repo = Arc::new(MockRepo::default());
    seed_wallet(&repo, "u1", 300);
    seed_wallet(&repo, "u2", 10);
    let res = service(repo.clone())
        .transfer(transfer_cmd("u1", "u2", 100, Some("merci")))
        .await
        .unwrap();
    assert_eq!(res.amount, 100);
    assert_eq!(res.from_balance, 200);
    assert_eq!(res.to_balance, 110);
    let transfers = repo.transfers.lock().unwrap();
    assert_eq!(transfers.len(), 1);
    assert_eq!(
        transfers[0],
        (
            "g1".into(),
            "u1".into(),
            "u2".into(),
            100,
            Some("merci".into())
        )
    );
}

#[tokio::test]
async fn transfer_creates_missing_wallets_with_starting_coins_first() {
    let repo = Arc::new(MockRepo::default());
    // Aucun wallet : les deux sont crees avec 100 coins, puis transfert 50.
    let res = service(repo.clone())
        .transfer(transfer_cmd("u1", "u2", 50, None))
        .await
        .unwrap();
    assert_eq!(res.from_balance, 50);
    assert_eq!(res.to_balance, 150);
}

#[tokio::test]
async fn transfer_rejects_self_transfer() {
    let repo = Arc::new(MockRepo::default());
    let err = service(repo.clone())
        .transfer(transfer_cmd("u1", "u1", 50, None))
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));
    assert!(repo.transfers.lock().unwrap().is_empty());
}

#[tokio::test]
async fn transfer_rejects_non_positive_amount() {
    let repo = Arc::new(MockRepo::default());
    let svc = service(repo.clone());
    assert!(svc
        .transfer(transfer_cmd("u1", "u2", 0, None))
        .await
        .is_err());
    assert!(svc
        .transfer(transfer_cmd("u1", "u2", -5, None))
        .await
        .is_err());
    assert!(repo.transfers.lock().unwrap().is_empty());
}

#[tokio::test]
async fn transfer_refuses_insufficient_balance_no_clamp() {
    let repo = Arc::new(MockRepo::default());
    seed_wallet(&repo, "u1", 99);
    seed_wallet(&repo, "u2", 0);
    let err = service(repo.clone())
        .transfer(transfer_cmd("u1", "u2", 100, None))
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));
    // Refus explicite : aucun transfert partiel, wallets intacts.
    assert!(repo.transfers.lock().unwrap().is_empty());
    assert_eq!(repo.wallets.lock().unwrap()["u1"].coins, 99);
}

#[tokio::test]
async fn transfer_rejects_amount_above_cap() {
    let repo = Arc::new(MockRepo::default());
    seed_wallet(&repo, "u1", i64::MAX);
    seed_wallet(&repo, "u2", 0);
    let err = service(repo.clone())
        .transfer(transfer_cmd(
            "u1",
            "u2",
            crate::nexus::domain::entities::wallet::MAX_WALLET_AMOUNT + 1,
            None,
        ))
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));
}

// ── history / leaderboard ──

#[tokio::test]
async fn history_applies_default_limit_and_offset() {
    let repo = Arc::new(MockRepo::default());
    service(repo.clone())
        .history("g1", "u1", None, None)
        .await
        .unwrap();
    assert_eq!(*repo.history_calls.lock().unwrap(), vec![(10, 0)]);
}

#[tokio::test]
async fn history_clamps_limit_and_floors_offset() {
    let repo = Arc::new(MockRepo::default());
    let svc = service(repo.clone());
    svc.history("g1", "u1", Some(999), Some(-4)).await.unwrap();
    svc.history("g1", "u1", Some(0), Some(20)).await.unwrap();
    assert_eq!(*repo.history_calls.lock().unwrap(), vec![(50, 0), (1, 20)]);
}

#[tokio::test]
async fn leaderboard_applies_default_and_clamped_limit() {
    let repo = Arc::new(MockRepo::default());
    let svc = service(repo.clone());
    svc.leaderboard("g1", None).await.unwrap();
    svc.leaderboard("g1", Some(3)).await.unwrap();
    svc.leaderboard("g1", Some(500)).await.unwrap();
    assert_eq!(*repo.leaderboard_calls.lock().unwrap(), vec![10, 3, 50]);
}
