//! Adapter Postgres du port `WalletRepository`.

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use platform_core::nexus::domain::entities::wallet::Wallet;
use platform_core::nexus::domain::entities::wallet::WalletMutation;
use platform_core::nexus::domain::entities::wallet::WalletTransaction;
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::wallet_repository::TransferOutcome;
use platform_core::nexus::ports::outbound::wallet_repository::WalletRepository;
use sqlx::PgPool;
use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;

use super::pg_err;

pub struct PgWalletRepository {
    pool: PgPool,
}

impl PgWalletRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type WalletRow = (String, String, String, i64, i64, i64);

fn wallet_from_row(row: WalletRow) -> Wallet {
    let (guild_id, user_id, username, coins, total_earned, total_spent) = row;
    let mut w = Wallet::new(guild_id, user_id);
    w.username = username;
    w.coins = coins;
    w.total_earned = total_earned;
    w.total_spent = total_spent;
    w
}

/// Verrouille (FOR UPDATE) et retourne le solde d'un wallet dans la tx.
async fn lock_wallet(
    tx: &mut Transaction<'_, Postgres>,
    guild_id: &str,
    user_id: &str,
) -> Result<i64, DomainError> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT coins FROM nexus_wallets
         WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(pg_err)?;
    row.map(|(c,)| c)
        .ok_or_else(|| DomainError::NotFound(format!("wallet {user_id} inexistant")))
}

#[async_trait]
impl WalletRepository for PgWalletRepository {
    async fn find(&self, guild_id: &str, user_id: &str) -> Result<Option<Wallet>, DomainError> {
        let row: Option<WalletRow> = sqlx::query_as(
            "SELECT guild_id, user_id, username, coins, total_earned, total_spent
             FROM nexus_wallets WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(wallet_from_row))
    }

    async fn starting_coins(&self, guild_id: &str) -> Result<Option<i64>, DomainError> {
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT starting_coins FROM nexus_guild_config WHERE guild_id = $1")
                .bind(guild_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok(row.map(|(c,)| c))
    }

    async fn save_with_transaction(
        &self,
        wallet: &Wallet,
        mutation: &WalletMutation,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        sqlx::query(
            "INSERT INTO nexus_wallets (guild_id, user_id, username, coins, total_earned, total_spent)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (guild_id, user_id) DO UPDATE SET
                username = CASE WHEN EXCLUDED.username <> '' THEN EXCLUDED.username
                                ELSE nexus_wallets.username END,
                coins = EXCLUDED.coins,
                total_earned = EXCLUDED.total_earned,
                total_spent = EXCLUDED.total_spent,
                updated_at = NOW()",
        )
        .bind(&wallet.guild_id)
        .bind(&wallet.user_id)
        .bind(&wallet.username)
        .bind(wallet.coins)
        .bind(wallet.total_earned)
        .bind(wallet.total_spent)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        sqlx::query(
            "INSERT INTO nexus_wallet_transactions
             (guild_id, user_id, amount, balance_after, source, description, reason)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&wallet.guild_id)
        .bind(&wallet.user_id)
        .bind(mutation.amount)
        .bind(mutation.balance_after)
        .bind(&mutation.source)
        .bind(&mutation.description)
        .bind(&mutation.reason)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)
    }

    async fn transfer_atomic(
        &self,
        guild_id: &str,
        from_user_id: &str,
        to_user_id: &str,
        amount: i64,
        reason: Option<&str>,
    ) -> Result<TransferOutcome, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Verrouillage ORDONNE des deux lignes (ordre lexicographique des
        // user_id) pour eviter les deadlocks entre transferts croises.
        let (first, second) = if from_user_id <= to_user_id {
            (from_user_id, to_user_id)
        } else {
            (to_user_id, from_user_id)
        };
        let first_coins = lock_wallet(&mut tx, guild_id, first).await?;
        let second_coins = lock_wallet(&mut tx, guild_id, second).await?;
        let from_coins = if first == from_user_id {
            first_coins
        } else {
            second_coins
        };

        // Re-verification sous verrou : REFUS explicite, jamais de clamp.
        if from_coins < amount {
            return Err(DomainError::Validation(format!(
                "Solde insuffisant ({from_coins} coins)"
            )));
        }

        let (from_balance,): (i64,) = sqlx::query_as(
            "UPDATE nexus_wallets
             SET coins = coins - $3, total_spent = total_spent + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2
             RETURNING coins",
        )
        .bind(guild_id)
        .bind(from_user_id)
        .bind(amount)
        .fetch_one(&mut *tx)
        .await
        .map_err(pg_err)?;

        let (to_balance,): (i64,) = sqlx::query_as(
            "UPDATE nexus_wallets
             SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2
             RETURNING coins",
        )
        .bind(guild_id)
        .bind(to_user_id)
        .bind(amount)
        .fetch_one(&mut *tx)
        .await
        .map_err(pg_err)?;

        let description = format!("Don de {from_user_id} a {to_user_id}");
        for (user_id, tx_amount, balance_after, source) in [
            (from_user_id, -amount, from_balance, "transfer_out"),
            (to_user_id, amount, to_balance, "transfer_in"),
        ] {
            sqlx::query(
                "INSERT INTO nexus_wallet_transactions
                 (guild_id, user_id, amount, balance_after, source, description, reason)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(guild_id)
            .bind(user_id)
            .bind(tx_amount)
            .bind(balance_after)
            .bind(source)
            .bind(&description)
            .bind(reason)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;
        }

        tx.commit().await.map_err(pg_err)?;
        Ok(TransferOutcome {
            from_balance,
            to_balance,
        })
    }

    async fn history(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WalletTransaction>, DomainError> {
        type TxRow = (
            Uuid,
            i64,
            i64,
            String,
            String,
            Option<String>,
            DateTime<Utc>,
        );
        let rows: Vec<TxRow> = sqlx::query_as(
            "SELECT id, amount, balance_after, source, description, reason, created_at
             FROM nexus_wallet_transactions
             WHERE guild_id = $1 AND user_id = $2
             ORDER BY created_at DESC
             LIMIT $3 OFFSET $4",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(
                |(id, amount, balance_after, source, description, reason, created_at)| {
                    WalletTransaction {
                        id,
                        guild_id: guild_id.to_string(),
                        user_id: user_id.to_string(),
                        amount,
                        balance_after,
                        source,
                        description,
                        reason,
                        created_at,
                    }
                },
            )
            .collect())
    }

    async fn leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<Wallet>, DomainError> {
        let rows: Vec<WalletRow> = sqlx::query_as(
            "SELECT guild_id, user_id, username, coins, total_earned, total_spent
             FROM nexus_wallets
             WHERE guild_id = $1
             ORDER BY coins DESC, user_id ASC
             LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(wallet_from_row).collect())
    }
}
