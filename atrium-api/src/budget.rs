//! Quotas persistants appliques avant chaque appel payant a DeepSeek.

use sqlx::{PgPool, Row};

use crate::AppConfig;

#[derive(Clone)]
pub struct BudgetGuard {
    pool: PgPool,
    cooldown_secs: i64,
    user_daily_limit: i32,
    global_daily_limit: i32,
}

impl BudgetGuard {
    pub fn new(config: &AppConfig) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: PgPool::connect_lazy(&config.rag_database_url)?,
            cooldown_secs: config.user_cooldown_secs.min(i64::MAX as u64) as i64,
            user_daily_limit: config.user_daily_limit.min(i32::MAX as u32) as i32,
            global_daily_limit: config.global_daily_limit.min(i32::MAX as u32) as i32,
        })
    }

    /// Retourne un message utilisateur quand l'appel doit etre bloque.
    /// Les lignes utilisateur et globale sont verrouillees dans une seule
    /// transaction : deux requetes paralleles ne peuvent pas depasser le cap.
    pub async fn check_and_record(
        &self,
        guild_id: &str,
        user_id: &str,
        interactive: bool,
    ) -> Result<Option<String>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // Les quotas sont journaliers : conserver une courte fenetre suffit
        // au diagnostic sans faire grossir la table indefiniment.
        sqlx::query("DELETE FROM atrium_ai_usage_users WHERE usage_date < CURRENT_DATE - 7")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM atrium_ai_usage_global WHERE usage_date < CURRENT_DATE - 7")
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "INSERT INTO atrium_ai_usage_users (usage_date, guild_id, user_id) \
             VALUES (CURRENT_DATE, $1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(guild_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        let user = sqlx::query(
            "SELECT request_count, \
                    COALESCE(EXTRACT(EPOCH FROM (now() - last_request_at))::bigint, 9223372036854775807::bigint) AS elapsed \
             FROM atrium_ai_usage_users \
             WHERE usage_date = CURRENT_DATE AND guild_id = $1 AND user_id = $2 \
             FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        let user_count: i32 = user.try_get("request_count")?;
        let elapsed: i64 = user.try_get("elapsed")?;

        if self.user_daily_limit > 0 && user_count >= self.user_daily_limit {
            tx.rollback().await?;
            return Ok(Some(format!(
                "Tu as atteint la limite de {} questions pour aujourd'hui. Réessaie demain.",
                self.user_daily_limit
            )));
        }
        if interactive && self.cooldown_secs > 0 && elapsed < self.cooldown_secs {
            let remaining = self.cooldown_secs - elapsed;
            tx.rollback().await?;
            return Ok(Some(format!(
                "Doucement 🙂 Attends encore {remaining} seconde(s) avant une nouvelle question."
            )));
        }

        sqlx::query(
            "INSERT INTO atrium_ai_usage_global (usage_date) VALUES (CURRENT_DATE) \
             ON CONFLICT DO NOTHING",
        )
        .execute(&mut *tx)
        .await?;
        let global_count: i32 = sqlx::query(
            "SELECT request_count FROM atrium_ai_usage_global \
             WHERE usage_date = CURRENT_DATE FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?
        .try_get("request_count")?;
        if self.global_daily_limit > 0 && global_count >= self.global_daily_limit {
            tx.rollback().await?;
            return Ok(Some(
                "Atrium a atteint son quota quotidien. Réessaie demain ou contacte l'équipe."
                    .to_owned(),
            ));
        }

        sqlx::query(
            "UPDATE atrium_ai_usage_users SET request_count = request_count + 1, \
             last_request_at = CASE WHEN $3 THEN now() ELSE last_request_at END \
             WHERE usage_date = CURRENT_DATE AND guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(interactive)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE atrium_ai_usage_global SET request_count = request_count + 1 \
             WHERE usage_date = CURRENT_DATE",
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(None)
    }
}
