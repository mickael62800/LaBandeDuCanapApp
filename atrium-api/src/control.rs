//! Etat active/desactive d'Atrium, persistant par serveur.
//!
//! La VALEUR fait desormais autorite dans `bot_guild_config` (cle `enabled` du
//! bot `atrium-bot`), comme tout autre reglage par serveur du depot.
//! `atrium_guild_settings` reste ecrite a chaque bascule, mais uniquement pour
//! sa trace `updated_by`/`updated_at` : elle repond a « qui a coupe Atrium, et
//! quand ? », question a laquelle `bot_guild_config` ne repond pas.

use sqlx::PgPool;

use crate::{
    guild_config::{self, ConfigDefaults},
    AppConfig,
};

#[derive(Clone)]
pub struct BotControlStore {
    pool: PgPool,
    /// Replis d'environnement. Seul `enabled` est lu ici, mais `from_map` les
    /// exige : les transporter evite d'avoir deux chemins de lecture de la
    /// config, donc deux endroits ou la semantique peut diverger.
    defaults: ConfigDefaults,
}

impl BotControlStore {
    pub fn new(pool: PgPool, config: &AppConfig) -> Self {
        Self {
            pool,
            defaults: ConfigDefaults {
                user_cooldown_secs: config.user_cooldown_secs.min(i64::MAX as u64) as i64,
                user_daily_limit: config.user_daily_limit.min(i32::MAX as u32) as i32,
                global_daily_limit: config.global_daily_limit.min(i32::MAX as u32) as i32,
            },
        }
    }

    /// Cle absente = DESACTIVE (fail-closed), comme partout dans le depot.
    ///
    /// Passe par `guild_config` plutot que de reimplementer le parsing : c'est
    /// exactement ainsi qu'une semantique de `enabled` finit par diverger d'un
    /// fichier a l'autre.
    pub async fn is_enabled(&self, guild_id: &str) -> Result<bool, sqlx::Error> {
        let raw = guild_config::load(&self.pool, guild_id).await?;
        Ok(guild_config::from_map(&raw, self.defaults).enabled)
    }

    /// Reglages bruts du serveur, tels qu'ils sont en base.
    ///
    /// Sert le RPC `GetGuildConfig` : atrium-bot n'a pas d'acces base et doit
    /// pourtant lire quelques cles par serveur. On renvoie la map brute plutot
    /// qu'un type structure — les defauts appartiennent a l'appelant, et un
    /// `GuildSettings` ici obligerait a etendre trois signatures a chaque
    /// nouvelle cle que seul le bot consomme.
    pub async fn raw_config(
        &self,
        guild_id: &str,
    ) -> Result<std::collections::HashMap<String, String>, sqlx::Error> {
        guild_config::load(&self.pool, guild_id).await
    }

    pub async fn set_enabled(
        &self,
        guild_id: &str,
        enabled: bool,
        actor_id: &str,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value) \
             VALUES ($1, $2, 'enabled', $3) \
             ON CONFLICT (guild_id, bot_name, config_key) \
             DO UPDATE SET config_value = EXCLUDED.config_value, updated_at = now()",
        )
        .bind(guild_id)
        .bind(guild_config::BOT_NAME)
        .bind(if enabled { "true" } else { "false" })
        .execute(&mut *tx)
        .await?;

        // Trace d'audit. Ecrite dans la meme transaction : un etat modifie sans
        // auteur enregistre serait pire que pas de trace du tout.
        sqlx::query(
            "INSERT INTO atrium_guild_settings (guild_id, enabled, updated_by) VALUES ($1, $2, $3) \
             ON CONFLICT (guild_id) DO UPDATE SET enabled = EXCLUDED.enabled, \
             updated_by = EXCLUDED.updated_by, updated_at = now()",
        )
        .bind(guild_id)
        .bind(enabled)
        .bind(actor_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await
    }
}
