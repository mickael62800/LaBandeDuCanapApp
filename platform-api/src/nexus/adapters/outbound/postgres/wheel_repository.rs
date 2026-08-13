//! Adapter Postgres du port `WheelRepository`.

use async_trait::async_trait;
use platform_core::nexus::domain::entities::wallet::{Wallet, WalletMutation};
use platform_core::nexus::domain::entities::wheel::{WheelCaseData, WheelSpin};
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::wheel_repository::WheelRepository;
use sqlx::PgPool;

use super::pg_err;

pub struct PgWheelRepository {
    pool: PgPool,
}

impl PgWheelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WheelRepository for PgWheelRepository {
    async fn try_claim(
        &self,
        guild_id: &str,
        user_id: &str,
        cooldown_hours: i64,
    ) -> Result<bool, DomainError> {
        // L'insertion CONDITIONNELLE fait office de verrou : le `NOT EXISTS`
        // et l'insertion sont evalues dans la meme instruction, donc deux
        // clics simultanes ne peuvent pas accorder deux tirages.
        //
        // `NOW()` cote base et non une date calculee en Rust : deux processus
        // aux horloges legerement decalees appliqueraient sinon des delais
        // differents.
        let res = sqlx::query(
            "INSERT INTO nexus_wheel_daily_claims (guild_id, user_id, day)
             SELECT $1, $2, CURRENT_DATE
             WHERE NOT EXISTS (
                 SELECT 1 FROM nexus_wheel_daily_claims
                 WHERE guild_id = $1 AND user_id = $2
                   AND claimed_at > NOW() - make_interval(hours => $3::int)
             )",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(cooldown_hours.clamp(1, 8760) as i32)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn has_claimed_recently(
        &self,
        guild_id: &str,
        user_id: &str,
        cooldown_hours: i64,
    ) -> Result<bool, DomainError> {
        // Meme reference de temps que `try_claim` : les deux doivent
        // s'accorder, sinon le bouton s'ouvre alors que le tirage sera refuse.
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM nexus_wheel_daily_claims
             WHERE guild_id = $1 AND user_id = $2
               AND claimed_at > NOW() - make_interval(hours => $3::int)
             LIMIT 1",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(cooldown_hours.clamp(1, 8760) as i32)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.is_some())
    }

    async fn log_spin(&self, spin: &WheelSpin) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO nexus_wheel_spin_log
             (id, guild_id, user_id, username, case_key, case_label, payout, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(spin.id)
        .bind(&spin.guild_id)
        .bind(&spin.user_id)
        .bind(&spin.username)
        .bind(&spin.case_key)
        .bind(&spin.case_label)
        .bind(spin.payout)
        .bind(spin.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_cases(&self, guild_id: &str) -> Result<Vec<WheelCaseData>, DomainError> {
        let rows: Vec<(String, String, i64, i32)> = sqlx::query_as(
            "SELECT key, label, payout, weight FROM nexus_wheel_cases
             WHERE guild_id = $1 ORDER BY position, key",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|(key, label, payout, weight)| WheelCaseData {
                key,
                label,
                payout,
                // Le CHECK garantit >= 1 ; la borne ici protege des lignes
                // ecrites a la main avant lui.
                weight: weight.max(1) as u32,
            })
            .collect())
    }

    async fn replace_cases(
        &self,
        guild_id: &str,
        cases: &[WheelCaseData],
    ) -> Result<(), DomainError> {
        // Une seule transaction : entre le DELETE et les INSERT, la roue
        // n'existe pas. Un tirage concurrent ne doit pas tomber dessus.
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        sqlx::query("DELETE FROM nexus_wheel_cases WHERE guild_id = $1")
            .bind(guild_id)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;
        for (position, case) in cases.iter().enumerate() {
            sqlx::query(
                "INSERT INTO nexus_wheel_cases (guild_id, key, label, payout, weight, position)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(guild_id)
            .bind(case.key.trim())
            .bind(case.label.trim())
            .bind(case.payout)
            .bind(case.weight.max(1) as i32)
            .bind(position as i32)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;
        }
        tx.commit().await.map_err(pg_err)
    }

    async fn execute_spin_transaction(
        &self,
        guild_id: &str,
        user_id: &str,
        cooldown_hours: i64,
        spin: &WheelSpin,
        wallet: &Wallet,
        mutation: Option<&WalletMutation>,
    ) -> Result<bool, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // 1. Essayer le claim
        let res = sqlx::query(
            "INSERT INTO nexus_wheel_daily_claims (guild_id, user_id, day)
             SELECT $1, $2, CURRENT_DATE
             WHERE NOT EXISTS (
                 SELECT 1 FROM nexus_wheel_daily_claims
                 WHERE guild_id = $1 AND user_id = $2
                   AND claimed_at > NOW() - make_interval(hours => $3::int)
             )",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(cooldown_hours.clamp(1, 8760) as i32)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        if res.rows_affected() == 0 {
            tx.rollback().await.map_err(pg_err)?;
            return Ok(false);
        }

        // 2. Toujours mettre à jour le wallet (pour sync le pseudo, même à 0)
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

        if let Some(mutation) = mutation {
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
        }

        // 3. Historiser le tirage
        sqlx::query(
            "INSERT INTO nexus_wheel_spin_log
             (id, guild_id, user_id, username, case_key, case_label, payout, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(spin.id)
        .bind(&spin.guild_id)
        .bind(&spin.user_id)
        .bind(&spin.username)
        .bind(&spin.case_key)
        .bind(&spin.case_label)
        .bind(spin.payout)
        .bind(spin.created_at)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(true)
    }
}
