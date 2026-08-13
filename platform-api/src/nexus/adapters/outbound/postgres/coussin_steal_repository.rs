use super::pg_err;
use async_trait::async_trait;
use platform_core::nexus::{
    domain::errors::DomainError, ports::outbound::coussin_steal_repository::CoussinStealRepository,
};
use sqlx::PgPool;

pub struct PgCoussinStealRepository {
    pool: PgPool,
}
impl PgCoussinStealRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl CoussinStealRepository for PgCoussinStealRepository {
    async fn balances(
        &self,
        guild: &str,
        thief: &str,
        victim: &str,
    ) -> Result<(i64, i64), DomainError> {
        // Le delai RESTANT, pas une duree en dur : le message annoncait
        // « 30 minutes » quel que soit le reglage du serveur, et restait faux
        // meme une seconde avant la fin du delai.
        let cooldown: Option<(f64,)> = sqlx::query_as("SELECT EXTRACT(EPOCH FROM (available_at - NOW())) FROM nexus_coussin_cooldowns WHERE guild_id=$1 AND user_id=$2 AND action='steal' AND available_at>NOW()").bind(guild).bind(thief).fetch_optional(&self.pool).await.map_err(pg_err)?;
        if let Some((secondes,)) = cooldown {
            let minutes = (secondes / 60.0).ceil().max(1.0) as i64;
            return Err(DomainError::RateLimited(format!(
                "tu as deja fouille recemment : reessaie dans {minutes} min"
            )));
        }
        let a: Option<(i64,)> =
            sqlx::query_as("SELECT coins FROM nexus_wallets WHERE guild_id=$1 AND user_id=$2")
                .bind(guild)
                .bind(thief)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;
        let b: Option<(i64,)> =
            sqlx::query_as("SELECT coins FROM nexus_wallets WHERE guild_id=$1 AND user_id=$2")
                .bind(guild)
                .bind(victim)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok((
            a.ok_or_else(|| DomainError::NotFound("wallet voleur".into()))?
                .0,
            b.ok_or_else(|| DomainError::NotFound("wallet cible".into()))?
                .0,
        ))
    }
    async fn transfer(
        &self,
        guild: &str,
        thief: &str,
        victim: &str,
        amount: i64,
        success: bool,
        cooldown_minutes: i64,
    ) -> Result<(), DomainError> {
        let (from, to) = if success {
            (victim, thief)
        } else {
            (thief, victim)
        };
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let debit=sqlx::query("UPDATE nexus_wallets SET coins=coins-$3,total_spent=total_spent+$3 WHERE guild_id=$1 AND user_id=$2 AND coins>=$3").bind(guild).bind(from).bind(amount).execute(&mut *tx).await.map_err(pg_err)?;
        if debit.rows_affected() != 1 {
            return Err(DomainError::Validation("solde insuffisant".into()));
        }
        sqlx::query("UPDATE nexus_wallets SET coins=coins+$3,total_earned=total_earned+$3 WHERE guild_id=$1 AND user_id=$2").bind(guild).bind(to).bind(amount).execute(&mut *tx).await.map_err(pg_err)?;
        if success {
            sqlx::query("UPDATE nexus_coussin_players SET total_stolen=total_stolen+$3 WHERE guild_id=$1 AND user_id=$2").bind(guild).bind(thief).bind(amount).execute(&mut *tx).await.map_err(pg_err)?;
        }
        sqlx::query("INSERT INTO nexus_coussin_cooldowns (guild_id,user_id,action,available_at) VALUES ($1,$2,'steal',NOW()+make_interval(mins => $3::int)) ON CONFLICT (guild_id,user_id,action) DO UPDATE SET available_at=EXCLUDED.available_at").bind(guild).bind(thief).bind(cooldown_minutes.clamp(0,10080) as i32).execute(&mut *tx).await.map_err(pg_err)?;

        // Trace du vol dans l'historique du portefeuille, pour les DEUX
        // parties. Sans elle, un vol ne laissait qu'un compteur agrege et un
        // solde qui bougeait : impossible de savoir qui avait pris quoi, ni
        // quand. Une victime voyait ses coins disparaitre sans explication.
        //
        // Ecrit dans la MEME transaction que le transfert : une trace qui
        // pourrait manquer alors que les coins ont bouge vaudrait moins que
        // pas de trace du tout.
        let (source, recit_debiteur, recit_crediteur) = if success {
            ("coussin_steal", "Vol subi", "Vol reussi")
        } else {
            ("coussin_steal_failed", "Vol rate", "Dedommagement")
        };

        for (user, montant, recit) in [
            (from, -amount, recit_debiteur),
            (to, amount, recit_crediteur),
        ] {
            sqlx::query(
                "INSERT INTO nexus_wallet_transactions
                     (guild_id, user_id, amount, balance_after, source, description)
                 SELECT $1, $2, $3, coins, $4, $5
                 FROM nexus_wallets WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(guild)
            .bind(user)
            .bind(montant)
            .bind(source)
            .bind(recit)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;
        }
        tx.commit().await.map_err(pg_err)
    }
}
