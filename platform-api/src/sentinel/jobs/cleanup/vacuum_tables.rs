//! Job : VACUUM ANALYZE periodique sur les tables les plus volumineuses.
//! Porte de cleanup-worker (Phase 1 fusion).

use sqlx::PgPool;
use tracing::{info, warn};

const TABLES: &[&str] = &[
    "voice_sessions",
    "audit_logs",
    "infractions",
    "ticket_messages",
    "user_activity_log",
    "logs",
];

pub async fn run(pool: &PgPool) -> Result<(), String> {
    // Sub-feature gate : vacuum_enabled (toggle UI sous cleanup).
    // VACUUM est global (DB-level, pas par guild), donc on regarde si
    // AU MOINS une guild a vacuum_enabled=true. Si aucune n'a explicitement
    // mis false ET la cle n'existe pas du tout -> on respecte le default
    // (true) du schema.
    let any_disabled: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM bot_guild_config \
         WHERE bot_name = 'cleanup' AND config_key = 'vacuum_enabled' \
           AND config_value IN ('true','1') LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    let any_explicit_false: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM bot_guild_config \
         WHERE bot_name = 'cleanup' AND config_key = 'vacuum_enabled' \
           AND config_value IN ('false','0') LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    // Si quelqu'un a actif -> run. Sinon si quelqu'un a explicit false et
    // personne actif -> skip. Sinon (aucune row) -> default true.
    if any_disabled.is_none() && any_explicit_false.is_some() {
        info!("VACUUM skip : vacuum_enabled=false sur toutes les guilds");
        return Ok(());
    }

    let mut errors = Vec::new();

    for table in TABLES {
        let start = std::time::Instant::now();
        let query = format!("VACUUM ANALYZE {table}");
        match sqlx::query(&query).execute(pool).await {
            Ok(_) => {
                let elapsed = start.elapsed();
                info!(
                    table,
                    duration_ms = elapsed.as_millis() as u64,
                    "VACUUM ANALYZE termine"
                );
            }
            Err(e) => {
                warn!(table, error = %e, "Erreur VACUUM ANALYZE");
                errors.push(format!("{table}: {e}"));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Erreurs VACUUM partielles: {}", errors.join("; ")))
    }
}
