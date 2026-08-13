use super::pg_err;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_core::nexus::{
    domain::errors::DomainError,
    ports::outbound::coussin_insurance_repository::{CoussinInsurance, CoussinInsuranceRepository},
};
use sqlx::PgPool;
pub struct PgCoussinInsuranceRepository {
    pool: PgPool,
}
impl PgCoussinInsuranceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl CoussinInsuranceRepository for PgCoussinInsuranceRepository {
    async fn buy(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
        cost: i64,
        duration_minutes: i64,
    ) -> Result<CoussinInsurance, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let (balance_after,):(i64,)=sqlx::query_as("UPDATE nexus_wallets SET coins=coins-$3,total_spent=total_spent+$3,updated_at=NOW() WHERE guild_id=$1 AND user_id=$2 AND coins>=$3 RETURNING coins").bind(guild_id).bind(user_id).bind(cost).fetch_optional(&mut *tx).await.map_err(pg_err)?.ok_or_else(||DomainError::Validation(format!("solde insuffisant ({cost} coins))").replace("))", ")")))?;
        sqlx::query("UPDATE nexus_coussin_insurances SET active=FALSE WHERE guild_id=$1 AND user_id=$2 AND active=TRUE").bind(guild_id).bind(user_id).execute(&mut *tx).await.map_err(pg_err)?;
        let (expires_at,):(DateTime<Utc>,)=sqlx::query_as("INSERT INTO nexus_coussin_insurances (guild_id,user_id,is_scam,expires_at) VALUES ($1,$2,$3,NOW()+make_interval(mins => $4::int)) RETURNING expires_at").bind(guild_id).bind(user_id).bind(is_scam).bind(duration_minutes.clamp(1,10080) as i32).fetch_one(&mut *tx).await.map_err(pg_err)?;
        sqlx::query("INSERT INTO nexus_wallet_transactions (guild_id,user_id,amount,balance_after,source,description) VALUES ($1,$2,$4,$3,'coussin_insurance','Garantie anti-tache Coussin Piégé')").bind(guild_id).bind(user_id).bind(balance_after).bind(-cost).execute(&mut *tx).await.map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(CoussinInsurance {
            is_scam,
            expires_at,
        })
    }
    async fn active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoussinInsurance>, DomainError> {
        let row:Option<(bool,DateTime<Utc>)>=sqlx::query_as("SELECT is_scam,expires_at FROM nexus_coussin_insurances WHERE guild_id=$1 AND user_id=$2 AND active=TRUE AND expires_at>NOW() ORDER BY expires_at DESC LIMIT 1").bind(guild_id).bind(user_id).fetch_optional(&self.pool).await.map_err(pg_err)?;
        Ok(row.map(|(is_scam, expires_at)| CoussinInsurance {
            is_scam,
            expires_at,
        }))
    }
}
