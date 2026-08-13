use super::pg_err;
use async_trait::async_trait;
use platform_core::nexus::{
    domain::{entities::coussin::PlayerClass, errors::DomainError},
    ports::outbound::coussin_repository::{
        CoussinBet, CoussinCombat, CoussinCombatResult, CoussinCombatSnapshot, CoussinPrime,
        CoussinProfile, CoussinProgress, CoussinRepository,
    },
};
use sqlx::PgPool;
pub struct PgCoussinRepository {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
struct ProfileRow {
    guild_id: String,
    user_id: String,
    username: String,
    class: String,
    level: i32,
    xp: i64,
    atk: i32,
    def: i32,
    hp_current: i32,
    hp_max: i32,
    coins: i64,
    stat_points: i32,
    title: String,
    total_wins: i32,
    total_losses: i32,
    total_draws: i32,
    total_stolen: i64,
    cowardice_count: i32,
    chaos_events: i32,
}
impl PgCoussinRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl CoussinRepository for PgCoussinRepository {
    async fn find_profile(
        &self,
        guild: &str,
        user: &str,
    ) -> Result<Option<CoussinProfile>, DomainError> {
        let row: Option<ProfileRow> = sqlx::query_as("SELECT p.guild_id,p.user_id,p.username,p.class,p.level,p.xp,p.atk,p.def,p.hp_current,p.hp_max,COALESCE(w.coins, 0) AS coins,p.stat_points,p.title,p.total_wins,p.total_losses,p.total_draws,p.total_stolen,p.cowardice_count,p.chaos_events FROM nexus_coussin_players p LEFT JOIN nexus_wallets w ON w.guild_id=p.guild_id AND w.user_id=p.user_id WHERE p.guild_id=$1 AND p.user_id=$2").bind(guild).bind(user).fetch_optional(&self.pool).await.map_err(pg_err)?;
        row.map(|row| {
            PlayerClass::parse(&row.class)
                .map(|class| CoussinProfile {
                    guild_id: row.guild_id,
                    user_id: row.user_id,
                    username: row.username,
                    class,
                    level: row.level,
                    xp: row.xp,
                    atk: row.atk,
                    def: row.def,
                    hp_current: row.hp_current,
                    hp_max: row.hp_max,
                    coins: row.coins,
                    stat_points: row.stat_points,
                    title: row.title,
                    total_wins: row.total_wins,
                    total_losses: row.total_losses,
                    total_draws: row.total_draws,
                    total_stolen: row.total_stolen,
                    cowardice_count: row.cowardice_count,
                    chaos_events: row.chaos_events,
                })
                .ok_or_else(|| DomainError::Internal("classe Coussin invalide".into()))
        })
        .transpose()
    }
    async fn list_bets(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<CoussinBet>, DomainError> {
        let rows: Vec<BetRow> = sqlx::query_as(
            "SELECT id, backed_id, amount, won, payout, created_at              FROM nexus_coussin_bets              WHERE guild_id = $1 AND bettor_id = $2              ORDER BY created_at DESC LIMIT $3",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| CoussinBet {
                id: r.id,
                backed_id: r.backed_id,
                amount: r.amount,
                won: r.won,
                payout: r.payout,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn list_primes(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<CoussinPrime>, DomainError> {
        // Posees PAR lui ou SUR lui : les deux le concernent, et savoir
        // qu'on a une prime sur la tete est meme l'information la plus utile.
        let rows: Vec<PrimeRow> = sqlx::query_as(
            "SELECT id, target_id, target_name, placed_by_id, placed_by_name,                     amount, claimed, claimed_by_id, created_at              FROM nexus_coussin_primes              WHERE guild_id = $1 AND (target_id = $2 OR placed_by_id = $2)              ORDER BY created_at DESC LIMIT $3",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| CoussinPrime {
                id: r.id,
                target_id: r.target_id,
                target_name: r.target_name,
                placed_by_id: r.placed_by_id,
                placed_by_name: r.placed_by_name,
                amount: r.amount,
                claimed: r.claimed,
                claimed_by_id: r.claimed_by_id,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn list_combat_history(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<CoussinCombatResult>, DomainError> {
        // Uniquement les combats RESOLUS : un defi en attente n'a ni
        // vainqueur ni recit, l'afficher dans un historique n'apprendrait
        // rien. Le joueur peut etre d'un cote comme de l'autre.
        let rows: Vec<CombatRow> = sqlx::query_as(
            "SELECT id, attacker_id, attacker_name, defender_id, defender_name,                     mise, winner_id, attacker_roll, defender_roll, chaos_event,                     special_attack, result_message, coins_transferred, resolved_at              FROM nexus_coussin_combats              WHERE guild_id = $1 AND (attacker_id = $2 OR defender_id = $2)                AND status = 'resolved'              ORDER BY resolved_at DESC NULLS LAST              LIMIT $3",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| CoussinCombatResult {
                id: r.id,
                attacker_id: r.attacker_id,
                attacker_name: r.attacker_name,
                defender_id: r.defender_id,
                defender_name: r.defender_name,
                mise: r.mise,
                winner_id: r.winner_id,
                attacker_roll: r.attacker_roll,
                defender_roll: r.defender_roll,
                chaos_event: r.chaos_event,
                special_attack: r.special_attack,
                result_message: r.result_message,
                coins_transferred: r.coins_transferred,
                resolved_at: r.resolved_at,
            })
            .collect())
    }

    async fn list_profiles(
        &self,
        guild: &str,
        limit: i64,
    ) -> Result<Vec<CoussinProfile>, DomainError> {
        let rows: Vec<ProfileRow> = sqlx::query_as(
            "SELECT p.guild_id,p.user_id,p.username,p.class,p.level,p.xp,p.atk,p.def,p.hp_current,p.hp_max,COALESCE(w.coins, 0) AS coins,p.stat_points,p.title,p.total_wins,p.total_losses,p.total_draws,p.total_stolen,p.cowardice_count,p.chaos_events FROM nexus_coussin_players p LEFT JOIN nexus_wallets w ON w.guild_id=p.guild_id AND w.user_id=p.user_id WHERE p.guild_id=$1 ORDER BY p.level DESC, p.xp DESC LIMIT $2",
        )
        .bind(guild)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        // Une classe illisible en base ne doit pas faire echouer tout le
        // classement : on ignore la ligne fautive plutot que d'avorter.
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                PlayerClass::parse(&row.class).map(|class| CoussinProfile {
                    guild_id: row.guild_id,
                    user_id: row.user_id,
                    username: row.username,
                    class,
                    level: row.level,
                    xp: row.xp,
                    atk: row.atk,
                    def: row.def,
                    hp_current: row.hp_current,
                    hp_max: row.hp_max,
                    coins: row.coins,
                    stat_points: row.stat_points,
                    title: row.title,
                    total_wins: row.total_wins,
                    total_losses: row.total_losses,
                    total_draws: row.total_draws,
                    total_stolen: row.total_stolen,
                    cowardice_count: row.cowardice_count,
                    chaos_events: row.chaos_events,
                })
            })
            .collect())
    }
    async fn create_profile(&self, p: &CoussinProfile) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        sqlx::query("INSERT INTO nexus_coussin_players (guild_id,user_id,username,class,level,xp,atk,def,hp_current,hp_max) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT (guild_id,user_id) DO NOTHING").bind(&p.guild_id).bind(&p.user_id).bind(&p.username).bind(p.class.as_str()).bind(p.level).bind(p.xp).bind(p.atk).bind(p.def).bind(p.hp_current).bind(p.hp_max).execute(&mut *tx).await.map_err(pg_err)?;
        // Coussin n'a pas de monnaie propre : un nouveau joueur obtient le
        // wallet Nexus normal (et sa configuration starting_coins), une fois.
        sqlx::query("INSERT INTO nexus_wallets (guild_id,user_id,username,coins) SELECT $1,$2,$3,COALESCE((SELECT starting_coins FROM nexus_guild_config WHERE guild_id=$1),100) ON CONFLICT (guild_id,user_id) DO UPDATE SET username=CASE WHEN EXCLUDED.username <> '' THEN EXCLUDED.username ELSE nexus_wallets.username END, updated_at=NOW()").bind(&p.guild_id).bind(&p.user_id).bind(&p.username).execute(&mut *tx).await.map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(())
    }
    async fn update_class(
        &self,
        guild_id: &str,
        user_id: &str,
        class: PlayerClass,
        atk: i32,
        def: i32,
        hp_max: i32,
        cooldown_minutes: i64,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let result = sqlx::query("UPDATE nexus_coussin_players SET class=$3, atk=$4, def=$5, hp_current=$6, hp_max=$6, class_changed_at=NOW(), updated_at=NOW() WHERE guild_id=$1 AND user_id=$2")
            .bind(guild_id).bind(user_id).bind(class.as_str()).bind(atk).bind(def).bind(hp_max).execute(&mut *tx).await.map_err(pg_err)?;
        if result.rows_affected() != 1 {
            return Err(DomainError::NotFound(format!("profil Coussin {user_id}")));
        }

        sqlx::query("INSERT INTO nexus_coussin_cooldowns (guild_id,user_id,action,available_at) VALUES ($1,$2,'class',NOW()+make_interval(mins => $3::int)) ON CONFLICT (guild_id,user_id,action) DO UPDATE SET available_at=EXCLUDED.available_at")
            .bind(guild_id).bind(user_id).bind(cooldown_minutes.clamp(0, 10080) as i32).execute(&mut *tx).await.map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(())
    }
    async fn spend_stat_point(
        &self,
        guild_id: &str,
        user_id: &str,
        stat: &str,
    ) -> Result<CoussinProfile, DomainError> {
        let column = match stat {
            "atk" => "atk",
            "def" => "def",
            _ => return Err(DomainError::Validation("stat invalide".into())),
        };
        let sql = format!("UPDATE nexus_coussin_players SET {column}={column}+1, stat_points=stat_points-1, hp_max=CASE WHEN $3='def' THEN hp_max+10 ELSE hp_max END, hp_current=CASE WHEN $3='def' THEN LEAST(hp_current+10,hp_max+10) ELSE hp_current END, updated_at=NOW() WHERE guild_id=$1 AND user_id=$2 AND stat_points > 0");
        let result = sqlx::query(&sql)
            .bind(guild_id)
            .bind(user_id)
            .bind(stat)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        if result.rows_affected() != 1 {
            return Err(DomainError::Validation(
                "aucun point de statistique disponible".into(),
            ));
        }
        self.find_profile(guild_id, user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("profil Coussin {user_id}")))
    }
    async fn create_combat(
        &self,
        guild_id: &str,
        channel_id: &str,
        attacker: &CoussinProfile,
        defender: &CoussinProfile,
        mise: i64,
        cooldown_minutes: i64,
    ) -> Result<CoussinCombat, DomainError> {
        if mise <= 0 {
            return Err(DomainError::Validation("mise invalide".into()));
        }
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        let row: (uuid::Uuid, String, String, String, i64, String) = sqlx::query_as("INSERT INTO nexus_coussin_combats (guild_id,channel_id,attacker_id,attacker_name,defender_id,defender_name,mise,expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,NOW()+INTERVAL '24 hours') RETURNING id,guild_id,attacker_id,defender_id,mise,status")
            .bind(guild_id).bind(channel_id).bind(&attacker.user_id).bind(&attacker.username).bind(&defender.user_id).bind(&defender.username).bind(mise).fetch_one(&mut *tx).await.map_err(pg_err)?;

        sqlx::query("INSERT INTO nexus_coussin_cooldowns (guild_id,user_id,action,available_at) VALUES ($1,$2,'combat',NOW()+make_interval(mins => $3::int)) ON CONFLICT (guild_id,user_id,action) DO UPDATE SET available_at=EXCLUDED.available_at")
            .bind(guild_id).bind(&attacker.user_id).bind(cooldown_minutes.clamp(0, 10080) as i32).execute(&mut *tx).await.map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;

        Ok(CoussinCombat {
            id: row.0,
            guild_id: row.1,
            attacker_id: row.2,
            defender_id: row.3,
            mise: row.4,
            status: row.5,
        })
    }
    async fn accept_combat(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let combat: Option<(String, String, String, i64)> = sqlx::query_as(
            "SELECT guild_id, attacker_id, defender_id, mise FROM nexus_coussin_combats WHERE id=$1 AND defender_id=$2 AND status='pending' AND (expires_at IS NULL OR expires_at > NOW()) FOR UPDATE",
        ).bind(id).bind(defender_id).fetch_optional(&mut *tx).await.map_err(pg_err)?;
        let Some((guild_id, attacker_id, defender_id, mise)) = combat else {
            return Ok(false);
        };
        // Lock both balances before accepting: neither participant can enter a
        // duel they are unable to settle.
        let balances: Vec<(String, i64)> = sqlx::query_as(
            "SELECT user_id, coins FROM nexus_wallets WHERE guild_id=$1 AND user_id IN ($2, $3) ORDER BY user_id FOR UPDATE",
        ).bind(&guild_id).bind(&attacker_id).bind(&defender_id).fetch_all(&mut *tx).await.map_err(pg_err)?;
        if balances.len() != 2 || balances.iter().any(|(_, coins)| *coins < mise) {
            return Err(DomainError::Validation(
                "coins insuffisants pour accepter ce defi".into(),
            ));
        }
        let result = sqlx::query(
            "UPDATE nexus_coussin_combats SET status='accepted' WHERE id=$1 AND status='pending'",
        )
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(result.rows_affected() == 1)
    }
    async fn refuse_combat(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let result: Option<(String,)> = sqlx::query_as("UPDATE nexus_coussin_combats SET status='refused', resolved_at=NOW() WHERE id=$1 AND defender_id=$2 AND status='pending' RETURNING guild_id")
            .bind(id).bind(defender_id).fetch_optional(&mut *tx).await.map_err(pg_err)?;
        let Some((guild_id,)) = result else {
            return Ok(false);
        };
        sqlx::query("UPDATE nexus_coussin_players SET cowardice_count=cowardice_count+1,updated_at=NOW() WHERE guild_id=$1 AND user_id=$2")
            .bind(guild_id).bind(defender_id).execute(&mut *tx).await.map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(true)
    }
    async fn resolution_snapshot(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<CoussinCombatSnapshot>, DomainError> {
        let row: Option<(uuid::Uuid, String, String, String, i64, String)> = sqlx::query_as("SELECT id,guild_id,attacker_id,defender_id,mise,status FROM nexus_coussin_combats WHERE id=$1 AND status='accepted'")
            .bind(id).fetch_optional(&self.pool).await.map_err(pg_err)?;
        let Some((id, guild_id, attacker_id, defender_id, mise, status)) = row else {
            return Ok(None);
        };
        let Some(attacker) = self.find_profile(&guild_id, &attacker_id).await? else {
            return Err(DomainError::NotFound(format!(
                "profil Coussin {attacker_id}"
            )));
        };
        let Some(defender) = self.find_profile(&guild_id, &defender_id).await? else {
            return Err(DomainError::NotFound(format!(
                "profil Coussin {defender_id}"
            )));
        };
        Ok(Some(CoussinCombatSnapshot {
            combat: CoussinCombat {
                id,
                guild_id,
                attacker_id,
                defender_id,
                mise,
                status,
            },
            attacker,
            defender,
        }))
    }
    async fn resolve_combat(
        &self,
        id: uuid::Uuid,
        winner_id: Option<&str>,
        attacker_roll: i32,
        defender_roll: i32,
        transferred: i64,
        attacker_hp: i32,
        defender_hp: i32,
        bet_payout_pct: i64,
        attacker_progress: Option<CoussinProgress>,
        defender_progress: Option<CoussinProgress>,
    ) -> Result<bool, DomainError> {
        // 1..=100 et non 1..=6 : le nombre de faces du de est reglable par
        // serveur. Cette borne reste un garde-fou contre une valeur aberrante,
        // pas une regle de jeu — la regle vit dans le domaine.
        if !(1..=100).contains(&attacker_roll)
            || !(1..=100).contains(&defender_roll)
            || transferred < 0
        {
            return Err(DomainError::Validation("resultat de duel invalide".into()));
        }
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let combat: Option<(String, String, String, i64)> = sqlx::query_as(
            "SELECT guild_id, attacker_id, defender_id, mise FROM nexus_coussin_combats WHERE id=$1 AND status='accepted' FOR UPDATE",
        ).bind(id).fetch_optional(&mut *tx).await.map_err(pg_err)?;
        let Some((guild_id, attacker_id, defender_id, mise)) = combat else {
            return Ok(false);
        };
        let valid_winner = winner_id.is_none()
            || winner_id == Some(attacker_id.as_str())
            || winner_id == Some(defender_id.as_str());
        if !valid_winner || transferred != if winner_id.is_some() { mise } else { 0 } {
            return Err(DomainError::Validation(
                "resultat de duel incoherent".into(),
            ));
        }
        if let Some(winner) = winner_id {
            let loser = if winner == attacker_id {
                defender_id.as_str()
            } else {
                attacker_id.as_str()
            };
            let debit = sqlx::query("UPDATE nexus_wallets SET coins=coins-$1, total_spent=total_spent+$1, updated_at=NOW() WHERE guild_id=$2 AND user_id=$3 AND coins >= $1")
                .bind(transferred).bind(&guild_id).bind(loser).execute(&mut *tx).await.map_err(pg_err)?;
            if debit.rows_affected() != 1 {
                return Err(DomainError::Validation(
                    "coins insuffisants pour regler ce duel".into(),
                ));
            }
            sqlx::query("UPDATE nexus_wallets SET coins=coins+$1, total_earned=total_earned+$1, updated_at=NOW() WHERE guild_id=$2 AND user_id=$3")
                .bind(transferred).bind(&guild_id).bind(winner).execute(&mut *tx).await.map_err(pg_err)?;
            let bounties: Vec<(i64,)> = sqlx::query_as("UPDATE nexus_coussin_primes SET claimed=TRUE,claimed_by_id=$3,claimed_at=NOW() WHERE guild_id=$1 AND target_id=$2 AND claimed=FALSE RETURNING amount")
                .bind(&guild_id).bind(loser).bind(winner).fetch_all(&mut *tx).await.map_err(pg_err)?;
            let bounty: i64 = bounties.into_iter().map(|(amount,)| amount).sum();
            if bounty > 0 {
                sqlx::query("UPDATE nexus_wallets SET coins=coins+$1,total_earned=total_earned+$1,updated_at=NOW() WHERE guild_id=$2 AND user_id=$3")
                    .bind(bounty).bind(&guild_id).bind(winner).execute(&mut *tx).await.map_err(pg_err)?;
            }
            sqlx::query("UPDATE nexus_coussin_players SET total_wins=total_wins+1 WHERE guild_id=$1 AND user_id=$2")
                .bind(&guild_id).bind(winner).execute(&mut *tx).await.map_err(pg_err)?;
            sqlx::query("UPDATE nexus_coussin_players SET total_losses=total_losses+1 WHERE guild_id=$1 AND user_id=$2")
                .bind(&guild_id).bind(loser).execute(&mut *tx).await.map_err(pg_err)?;
            for (user_id, amount, source) in [
                (loser, -transferred, "coussin_loss"),
                (winner, transferred, "coussin_win"),
            ] {
                let (balance_after,): (i64,) = sqlx::query_as(
                    "SELECT coins FROM nexus_wallets WHERE guild_id=$1 AND user_id=$2",
                )
                .bind(&guild_id)
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(pg_err)?;
                sqlx::query("INSERT INTO nexus_wallet_transactions (guild_id,user_id,amount,balance_after,source,description) VALUES ($1,$2,$3,$4,$5,'Coussin Piégé')").bind(&guild_id).bind(user_id).bind(amount).bind(balance_after).bind(source).execute(&mut *tx).await.map_err(pg_err)?;
            }
            let winners: Vec<(String, i64)> = sqlx::query_as("UPDATE nexus_coussin_bets SET won=TRUE,payout=amount*$3/100 WHERE combat_id=$1 AND backed_id=$2 RETURNING bettor_id,payout")
                .bind(id).bind(winner).bind(bet_payout_pct.clamp(100, 1000)).fetch_all(&mut *tx).await.map_err(pg_err)?;
            sqlx::query("UPDATE nexus_coussin_bets SET won=FALSE,payout=0 WHERE combat_id=$1 AND backed_id<>$2")
                .bind(id).bind(winner).execute(&mut *tx).await.map_err(pg_err)?;
            for (bettor, payout) in winners {
                sqlx::query("UPDATE nexus_wallets SET coins=coins+$3,total_earned=total_earned+$3 WHERE guild_id=$1 AND user_id=$2").bind(&guild_id).bind(bettor).bind(payout).execute(&mut *tx).await.map_err(pg_err)?;
            }
        } else {
            sqlx::query("UPDATE nexus_coussin_players SET total_draws=total_draws+1 WHERE guild_id=$1 AND user_id IN ($2, $3)")
                .bind(&guild_id).bind(&attacker_id).bind(&defender_id).execute(&mut *tx).await.map_err(pg_err)?;
            let refunds: Vec<(String, i64)> = sqlx::query_as("UPDATE nexus_coussin_bets SET won=FALSE,payout=amount WHERE combat_id=$1 RETURNING bettor_id,payout").bind(id).fetch_all(&mut *tx).await.map_err(pg_err)?;
            for (bettor, payout) in refunds {
                sqlx::query("UPDATE nexus_wallets SET coins=coins+$3,total_earned=total_earned+$3 WHERE guild_id=$1 AND user_id=$2").bind(&guild_id).bind(bettor).bind(payout).execute(&mut *tx).await.map_err(pg_err)?;
            }
        }
        sqlx::query("UPDATE nexus_coussin_players SET hp_current=$3,updated_at=NOW() WHERE guild_id=$1 AND user_id=$2")
            .bind(&guild_id).bind(&attacker_id).bind(attacker_hp.max(0)).execute(&mut *tx).await.map_err(pg_err)?;
        sqlx::query("UPDATE nexus_coussin_players SET hp_current=$3,updated_at=NOW() WHERE guild_id=$1 AND user_id=$2")
            .bind(&guild_id).bind(&defender_id).bind(defender_hp.max(0)).execute(&mut *tx).await.map_err(pg_err)?;

        if let Some(p) = attacker_progress {
            sqlx::query("UPDATE nexus_coussin_players SET xp=$3,level=$4,stat_points=$5,title=$6,updated_at=NOW() WHERE guild_id=$1 AND user_id=$2").bind(&guild_id).bind(&attacker_id).bind(p.xp).bind(p.level).bind(p.stat_points).bind(&p.title).execute(&mut *tx).await.map_err(pg_err)?;
        }
        if let Some(p) = defender_progress {
            sqlx::query("UPDATE nexus_coussin_players SET xp=$3,level=$4,stat_points=$5,title=$6,updated_at=NOW() WHERE guild_id=$1 AND user_id=$2").bind(&guild_id).bind(&defender_id).bind(p.xp).bind(p.level).bind(p.stat_points).bind(&p.title).execute(&mut *tx).await.map_err(pg_err)?;
        }

        let result = sqlx::query("UPDATE nexus_coussin_combats SET status='resolved', winner_id=$2, attacker_roll=$3, defender_roll=$4, coins_transferred=$5, resolved_at=NOW() WHERE id=$1 AND status='accepted'")
            .bind(id).bind(winner_id).bind(attacker_roll).bind(defender_roll).bind(transferred).execute(&mut *tx).await.map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(result.rows_affected() == 1)
    }
}

#[derive(sqlx::FromRow)]
struct CombatRow {
    id: uuid::Uuid,
    attacker_id: String,
    attacker_name: String,
    defender_id: String,
    defender_name: String,
    mise: i64,
    winner_id: Option<String>,
    attacker_roll: Option<i32>,
    defender_roll: Option<i32>,
    chaos_event: Option<String>,
    special_attack: Option<String>,
    result_message: Option<String>,
    coins_transferred: i64,
    resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(sqlx::FromRow)]
struct BetRow {
    id: uuid::Uuid,
    backed_id: String,
    amount: i64,
    won: Option<bool>,
    payout: i64,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct PrimeRow {
    id: uuid::Uuid,
    target_id: String,
    target_name: String,
    placed_by_id: String,
    placed_by_name: String,
    amount: i64,
    claimed: bool,
    claimed_by_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}
