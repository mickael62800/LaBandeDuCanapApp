use super::pg_err;
use async_trait::async_trait;
use platform_core::nexus::{
    domain::errors::DomainError,
    ports::outbound::coussin_steal_repository::{CoussinStealRepository, StealAttempt},
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

    async fn settlement_balances(
        &self,
        guild: &str,
        thief: &str,
        victim: &str,
    ) -> Result<(i64, i64), DomainError> {
        let row: Option<(i64, i64)> = sqlx::query_as(
            "SELECT                  (SELECT coins FROM nexus_wallets WHERE guild_id=$1 AND user_id=$2),                  (SELECT coins FROM nexus_wallets WHERE guild_id=$1 AND user_id=$3)",
        )
        .bind(guild)
        .bind(thief)
        .bind(victim)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        row.ok_or_else(|| DomainError::NotFound("porte-monnaie introuvable".into()))
    }

    async fn open_attempt(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        channel_id: &str,
        defense_window_seconds: i64,
    ) -> Result<StealAttempt, DomainError> {
        // `ON CONFLICT DO NOTHING` sur l'index partiel : une fouille du meme
        // voleur sur la meme victime est deja ouverte. Enchainer la commande
        // ouvrirait sinon dix fenetres simultanees sur la meme personne.
        let row: Option<(uuid::Uuid, String)> = sqlx::query_as(
            "INSERT INTO nexus_coussin_steal_attempts                  (guild_id, thief_id, victim_id, channel_id, expires_at)              VALUES ($1, $2, $3, $4, NOW() + make_interval(secs => $5::double precision))              ON CONFLICT DO NOTHING              RETURNING id, to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')",
        )
        .bind(guild_id)
        .bind(thief_id)
        .bind(victim_id)
        .bind(channel_id)
        .bind(defense_window_seconds.clamp(10, 600) as f64)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        let Some((id, expires_at)) = row else {
            return Err(DomainError::Conflict(
                "tu fouilles deja les coussins de cette personne".into(),
            ));
        };

        Ok(StealAttempt {
            id,
            guild_id: guild_id.to_string(),
            thief_id: thief_id.to_string(),
            victim_id: victim_id.to_string(),
            channel_id: channel_id.to_string(),
            message_id: None,
            expires_at,
        })
    }

    async fn attach_message(
        &self,
        attempt_id: uuid::Uuid,
        message_id: &str,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE nexus_coussin_steal_attempts SET message_id=$2 WHERE id=$1")
            .bind(attempt_id)
            .bind(message_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn claim_attempt(
        &self,
        attempt_id: uuid::Uuid,
        by_victim: Option<&str>,
    ) -> Result<Option<StealAttempt>, DomainError> {
        // Le passage a 'resolved' se fait DANS la requete de lecture : la
        // victime qui clique a la derniere seconde et le job qui passe au meme
        // instant ne peuvent pas resoudre la meme fouille deux fois.
        //
        // La victime ne peut reclamer que sa propre fouille, et seulement
        // avant l'echeance : au-dela, son absence a deja valu reponse.
        let row: Option<(uuid::Uuid, String, String, String, String, Option<String>, String)> =
            sqlx::query_as(
                "UPDATE nexus_coussin_steal_attempts SET status='resolved', resolved_at=NOW()                  WHERE id=$1 AND status='pending'                    AND ($2::text IS NULL OR (victim_id = $2 AND expires_at > NOW()))                  RETURNING id, guild_id, thief_id, victim_id, channel_id, message_id,                      to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')",
            )
            .bind(attempt_id)
            .bind(by_victim)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(row.map(
            |(id, guild_id, thief_id, victim_id, channel_id, message_id, expires_at)| {
                StealAttempt {
                    id,
                    guild_id,
                    thief_id,
                    victim_id,
                    channel_id,
                    message_id,
                    expires_at,
                }
            },
        ))
    }

    async fn claim_expired_attempts(&self, limit: i64) -> Result<Vec<StealAttempt>, DomainError> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, Option<String>, String)> =
            sqlx::query_as(
                "UPDATE nexus_coussin_steal_attempts SET status='resolved', resolved_at=NOW()                  WHERE id IN (                      SELECT id FROM nexus_coussin_steal_attempts                      WHERE status='pending' AND expires_at <= NOW()                      ORDER BY expires_at ASC LIMIT $1                  )                  RETURNING id, guild_id, thief_id, victim_id, channel_id, message_id,                      to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')",
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(
                |(id, guild_id, thief_id, victim_id, channel_id, message_id, expires_at)| {
                    StealAttempt {
                        id,
                        guild_id,
                        thief_id,
                        victim_id,
                        channel_id,
                        message_id,
                        expires_at,
                    }
                },
            )
            .collect())
    }

    async fn record_outcome(
        &self,
        attempt_id: uuid::Uuid,
        defended: bool,
        success: bool,
        amount: i64,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE nexus_coussin_steal_attempts              SET defended=$2, success=$3, amount=$4 WHERE id=$1",
        )
        .bind(attempt_id)
        .bind(defended)
        .bind(success)
        .bind(amount)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
