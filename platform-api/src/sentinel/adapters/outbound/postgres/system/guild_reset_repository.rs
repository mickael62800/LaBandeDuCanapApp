use async_trait::async_trait;
use sqlx::PgPool;

use super::super::pg_err_ctx;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::system::guild_reset_repository::{
    GuildResetRepository, ResetDiscordContext,
};

/// Tables a NE JAMAIS effacer lors d'un reset par serveur :
/// - `guilds` : enregistrement du serveur (reste visible dans le dashboard)
/// - RBAC : l'owner garde son acces
/// - `bot_definitions` : metadata globale (non guild-scopee)
/// - `guild_snapshots` : les SAUVEGARDES sont le filet de securite ; un reset ne
///   doit PAS les detruire (sinon on perd la possibilite de revenir en arriere
///   apres un reset accidentel/regrette). Note : `pending_role_grants` (grants
///   en attente d'un restore anterieur) N'est PAS exclu — apres un reset ces
///   grants referencent des roles disparus, donc les purger est correct.
const EXCLUDED_TABLES: &[&str] = &[
    "guilds",
    "api_user_guilds",
    "api_users",
    "bot_definitions",
    "guild_snapshots",
];

pub struct PgGuildResetRepository {
    pool: PgPool,
}

impl PgGuildResetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx("guild_reset", e)
}

#[async_trait]
impl GuildResetRepository for PgGuildResetRepository {
    async fn guild_name(&self, guild_id: &str) -> Result<Option<String>, DomainError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT name FROM guilds WHERE guild_id = $1")
            .bind(guild_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(|(n,)| n))
    }

    async fn collect_discord_context(
        &self,
        guild_id: &str,
    ) -> Result<ResetDiscordContext, DomainError> {
        // Role de quarantaine (config security-bot).
        let quarantine_role_id: Option<(String,)> = sqlx::query_as(
            "SELECT config_value FROM bot_guild_config \
             WHERE guild_id = $1 AND bot_name = 'security-bot' AND config_key = 'quarantine_role_id'",
        )
        .bind(guild_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        // Roles temporaires poses par le bot.
        let temp_roles: Vec<(String,)> =
            sqlx::query_as("SELECT DISTINCT role_id FROM temp_roles WHERE guild_id = $1")
                .bind(guild_id)
                .fetch_all(&self.pool)
                .await
                .map_err(pg_err)?;

        Ok(ResetDiscordContext {
            quarantine_role_id: quarantine_role_id
                .map(|(v,)| v)
                .filter(|v| !v.trim().is_empty()),
            temp_role_ids: temp_roles.into_iter().map(|(r,)| r).collect(),
        })
    }

    async fn wipe_guild(&self, guild_id: &str) -> Result<Vec<(String, u64)>, DomainError> {
        // Decouvre dynamiquement toutes les tables guild-scopees (robuste : couvre
        // les tables futures sans maintenance d'une liste codee en dur).
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT table_name FROM information_schema.columns \
             WHERE column_name = 'guild_id' AND table_schema = 'public' \
             ORDER BY table_name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        let mut remaining: Vec<String> = rows
            .into_iter()
            .map(|(t,)| t)
            .filter(|t| !EXCLUDED_TABLES.contains(&t.as_str()))
            .collect();

        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let mut summary: Vec<(String, u64)> = Vec::new();

        // Multi-passes avec SAVEPOINT : une table qui echoue (contrainte FK vers
        // une table pas encore videe) est reessayee au tour suivant. Converge
        // sans connaitre le graphe de dependances. ON DELETE CASCADE gere le reste.
        loop {
            let mut still_failed: Vec<String> = Vec::new();
            let mut progressed = false;

            for table in &remaining {
                sqlx::query("SAVEPOINT wipe_sp")
                    .execute(&mut *tx)
                    .await
                    .map_err(pg_err)?;
                // Nom de table issu du catalogue (jamais d'input utilisateur) -> quote.
                let sql = format!("DELETE FROM \"{}\" WHERE guild_id = $1", table);
                match sqlx::query(&sql).bind(guild_id).execute(&mut *tx).await {
                    Ok(res) => {
                        sqlx::query("RELEASE SAVEPOINT wipe_sp")
                            .execute(&mut *tx)
                            .await
                            .map_err(pg_err)?;
                        progressed = true;
                        summary.push((table.clone(), res.rows_affected()));
                    }
                    Err(_) => {
                        sqlx::query("ROLLBACK TO SAVEPOINT wipe_sp")
                            .execute(&mut *tx)
                            .await
                            .map_err(pg_err)?;
                        still_failed.push(table.clone());
                    }
                }
            }

            if still_failed.is_empty() || !progressed {
                if !still_failed.is_empty() {
                    tracing::warn!(
                        guild_id,
                        tables = ?still_failed,
                        "Reset guild : certaines tables n'ont pu etre videes (FK insoluble) -- ignorees"
                    );
                }
                break;
            }
            remaining = still_failed;
        }

        tx.commit().await.map_err(pg_err)?;
        Ok(summary)
    }
}
