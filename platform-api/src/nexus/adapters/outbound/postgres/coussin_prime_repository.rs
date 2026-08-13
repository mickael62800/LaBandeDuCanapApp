use super::pg_err;
use async_trait::async_trait;
use platform_core::nexus::{
    domain::errors::DomainError, ports::outbound::coussin_prime_repository::CoussinPrimeRepository,
};
use sqlx::PgPool;
pub struct PgCoussinPrimeRepository {
    pool: PgPool,
}
impl PgCoussinPrimeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl CoussinPrimeRepository for PgCoussinPrimeRepository {
    async fn place(
        &self,
        guild_id: &str,
        target_id: &str,
        target_name: &str,
        placer_id: &str,
        placer_name: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let debit=sqlx::query("UPDATE nexus_wallets SET coins=coins-$3,total_spent=total_spent+$3 WHERE guild_id=$1 AND user_id=$2 AND coins >= $3").bind(guild_id).bind(placer_id).bind(amount).execute(&mut *tx).await.map_err(pg_err)?;
        if debit.rows_affected() != 1 {
            return Err(DomainError::Validation("solde insuffisant".into()));
        }
        sqlx::query("INSERT INTO nexus_coussin_primes (guild_id,target_id,target_name,placed_by_id,placed_by_name,amount) VALUES ($1,$2,$3,$4,$5,$6)").bind(guild_id).bind(target_id).bind(target_name).bind(placer_id).bind(placer_name).bind(amount).execute(&mut *tx).await.map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)
    }
}
