use super::pg_err;
use async_trait::async_trait;
use platform_core::nexus::{
    domain::errors::DomainError, ports::outbound::coussin_bet_repository::CoussinBetRepository,
};
use sqlx::PgPool;
pub struct PgCoussinBetRepository {
    pool: PgPool,
}
impl PgCoussinBetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl CoussinBetRepository for PgCoussinBetRepository {
    async fn place(
        &self,
        guild: &str,
        combat: uuid::Uuid,
        bettor: &str,
        name: &str,
        backed: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let valid:Option<(String,String)>=sqlx::query_as("SELECT attacker_id,defender_id FROM nexus_coussin_combats WHERE id=$1 AND guild_id=$2 AND status='accepted' FOR UPDATE").bind(combat).bind(guild).fetch_optional(&mut *tx).await.map_err(pg_err)?;
        let Some((a, d)) = valid else {
            return Err(DomainError::Validation(
                "combat non disponible pour les paris".into(),
            ));
        };
        if backed != a && backed != d {
            return Err(DomainError::Validation("combattant invalide".into()));
        }
        let debit=sqlx::query("UPDATE nexus_wallets SET coins=coins-$3,total_spent=total_spent+$3 WHERE guild_id=$1 AND user_id=$2 AND coins >=$3").bind(guild).bind(bettor).bind(amount).execute(&mut *tx).await.map_err(pg_err)?;
        if debit.rows_affected() != 1 {
            return Err(DomainError::Validation("solde insuffisant".into()));
        }
        sqlx::query("INSERT INTO nexus_coussin_bets (guild_id,combat_id,bettor_id,bettor_name,backed_id,amount) VALUES ($1,$2,$3,$4,$5,$6)").bind(guild).bind(combat).bind(bettor).bind(name).bind(backed).bind(amount).execute(&mut *tx).await.map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)
    }
}
