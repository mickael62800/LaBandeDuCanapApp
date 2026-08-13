use super::pg_err;
use async_trait::async_trait;
use platform_core::nexus::{
    domain::errors::DomainError,
    ports::outbound::coussin_inventory_repository::{CoussinInventoryRepository, InventoryItem},
};
use sqlx::PgPool;

pub struct PgCoussinInventoryRepository {
    pool: PgPool,
}
impl PgCoussinInventoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl CoussinInventoryRepository for PgCoussinInventoryRepository {
    async fn list(&self, guild_id: &str, user_id: &str) -> Result<Vec<InventoryItem>, DomainError> {
        let rows: Vec<(String, i32)> = sqlx::query_as("SELECT item_key, quantity FROM nexus_coussin_inventory WHERE guild_id=$1 AND user_id=$2 AND quantity > 0 ORDER BY item_key")
            .bind(guild_id).bind(user_id).fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|(item_key, quantity)| InventoryItem { item_key, quantity })
            .collect())
    }
    async fn buy(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        price: i64,
    ) -> Result<i64, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let (balance_after,): (i64,) = sqlx::query_as("UPDATE nexus_wallets SET coins=coins-$3,total_spent=total_spent+$3,updated_at=NOW() WHERE guild_id=$1 AND user_id=$2 AND coins >= $3 RETURNING coins")
            .bind(guild_id).bind(user_id).bind(price).fetch_optional(&mut *tx).await.map_err(pg_err)?
            .ok_or_else(|| DomainError::Validation("solde insuffisant".into()))?;
        sqlx::query("INSERT INTO nexus_coussin_inventory (guild_id,user_id,item_key,quantity) VALUES ($1,$2,$3,1) ON CONFLICT (guild_id,user_id,item_key) DO UPDATE SET quantity=nexus_coussin_inventory.quantity+1")
            .bind(guild_id).bind(user_id).bind(item_key).execute(&mut *tx).await.map_err(pg_err)?;
        sqlx::query("INSERT INTO nexus_wallet_transactions (guild_id,user_id,amount,balance_after,source,description) VALUES ($1,$2,$3,$4,'coussin_shop','Achat Coussin Piégé')")
            .bind(guild_id).bind(user_id).bind(-price).bind(balance_after).execute(&mut *tx).await.map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(balance_after)
    }
}
