//! Quotas persistants appliques avant chaque appel payant a DeepSeek.
//!
//! Les limites sont lues PAR SERVEUR dans `bot_guild_config` a chaque
//! verification, avec repli sur les valeurs d'environnement du demarrage. Elles
//! etaient auparavant figees dans le processus : identiques pour tous les
//! serveurs, et un redemarrage etait necessaire pour en changer une.

use sqlx::{PgPool, Row};

use crate::{
    guild_config::{self, ConfigDefaults},
    AppConfig,
};

#[derive(Clone)]
pub struct BudgetGuard {
    pool: PgPool,
    defaults: ConfigDefaults,
}

/// Photographie des quotas pour l'administration.
///
/// Les limites renvoyees sont celles REELLEMENT appliquees au serveur demande :
/// sa configuration (`bot_guild_config`) si elle existe, sinon le repli lu dans
/// l'environnement au demarrage. L'ecran d'administration affiche donc la
/// valeur qui s'applique, pas une valeur theorique.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BudgetStats {
    /// Requetes consommees aujourd'hui, toutes guildes confondues.
    pub global_used_today: i64,
    /// Requetes consommees aujourd'hui par la guilde demandee.
    pub guild_used_today: i64,
    /// Membres distincts ayant sollicite Atrium aujourd'hui dans cette guilde.
    pub guild_active_users_today: i64,
    pub global_daily_limit: i32,
    pub user_daily_limit: i32,
    pub user_cooldown_secs: i64,
}

impl BudgetGuard {
    /// Lecture seule des compteurs du jour. N'incremente rien, contrairement a
    /// `check_and_record` : un ecran d'administration qui consomme du quota en
    /// s'affichant serait un piege.
    pub async fn stats(&self, guild_id: &str) -> Result<BudgetStats, sqlx::Error> {
        let global_used_today: i64 = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT request_count FROM atrium_ai_usage_global WHERE usage_date = CURRENT_DATE",
        )
        .fetch_optional(&self.pool)
        .await?
        .flatten()
        .unwrap_or(0)
        .into();

        let row = sqlx::query(
            "SELECT COALESCE(SUM(request_count), 0)::bigint AS used, \
                    COUNT(*)::bigint AS actives \
             FROM atrium_ai_usage_users \
             WHERE usage_date = CURRENT_DATE AND guild_id = $1",
        )
        .bind(guild_id)
        .fetch_one(&self.pool)
        .await?;

        // Les limites renvoyees sont celles REELLEMENT appliquees a ce serveur :
        // sa configuration si elle existe, le repli d'environnement sinon.
        let limits = guild_config::settings(&self.pool, guild_id, self.defaults).await?;

        Ok(BudgetStats {
            global_used_today,
            guild_used_today: row.try_get("used")?,
            guild_active_users_today: row.try_get("actives")?,
            global_daily_limit: limits.global_daily_limit,
            user_daily_limit: limits.user_daily_limit,
            user_cooldown_secs: limits.user_cooldown_secs,
        })
    }

    pub fn new(config: &AppConfig) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: PgPool::connect_lazy(&config.rag_database_url)?,
            defaults: ConfigDefaults {
                user_cooldown_secs: config.user_cooldown_secs.min(i64::MAX as u64) as i64,
                user_daily_limit: config.user_daily_limit.min(i32::MAX as u32) as i32,
                global_daily_limit: config.global_daily_limit.min(i32::MAX as u32) as i32,
            },
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
        // Limites du serveur, lues avant d'ouvrir la transaction : c'est une
        // lecture independante des compteurs, et la garder hors transaction
        // raccourcit d'autant la duree des verrous `FOR UPDATE` poses plus bas.
        let limits = guild_config::settings(&self.pool, guild_id, self.defaults).await?;

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

        if limits.user_daily_limit > 0 && user_count >= limits.user_daily_limit {
            tx.rollback().await?;
            return Ok(Some(format!(
                "Tu as atteint la limite de {} questions pour aujourd'hui. Réessaie demain.",
                limits.user_daily_limit
            )));
        }
        if interactive && limits.user_cooldown_secs > 0 && elapsed < limits.user_cooldown_secs {
            let remaining = limits.user_cooldown_secs - elapsed;
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
        if limits.global_daily_limit > 0 && global_count >= limits.global_daily_limit {
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
