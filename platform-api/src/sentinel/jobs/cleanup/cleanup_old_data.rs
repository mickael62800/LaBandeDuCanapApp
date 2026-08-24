//! Job : suppression des anciennes donnees selon les retentions
//! configurees (voice_sessions, logs, audit_logs, ticket_messages, ...).
//!
//! Porte de cleanup-worker (Phase 1 fusion) — logique inchangee, seul
//! le chemin d'import a ete adapte pour pointer vers la config du
//! sentinel-worker.

use sqlx::PgPool;
use tracing::{info, warn};

use super::CleanupConfig;

const DELETE_OLD_LOGS_SQL: &str =
    "DELETE FROM ONLY logs WHERE \"timestamp\" < NOW() - make_interval(days => $1::int)";

fn record_cleanup_success(table: &'static str, rows: u64) {
    metrics::counter!("cleanup_rows_total", "table" => table).increment(rows);
}

fn record_cleanup_error(table: &'static str) {
    metrics::counter!("cleanup_errors_total", "table" => table).increment(1);
}

async fn delete_old_logs(pool: &PgPool, days: i32) -> Result<u64, sqlx::Error> {
    sqlx::query(DELETE_OLD_LOGS_SQL)
        .bind(days)
        .execute(pool)
        .await
        .map(|result| result.rows_affected())
}

/// `voice_sessions` n'a PAS de colonne `created_at` : la table porte
/// `started_at` et `ended_at` (voir `idx_voice_sessions_started`). Le DELETE
/// visait `created_at` et echouait donc a chaque passage du job, avale par le
/// `warn!` qui l'entoure : aucune session vocale n'a jamais ete purgee, et la
/// retention configuree dans le module `cleanup` n'avait aucun effet.
///
/// La retention se compte depuis le DEBUT de la session : c'est la colonne
/// indexee, et une session commencee il y a quatre-vingt-dix jours est ancienne
/// quelle que soit sa duree.
const DELETE_OLD_VOICE_SESSIONS_SQL: &str =
    "DELETE FROM voice_sessions WHERE started_at < NOW() - make_interval(days => $1::int)";

async fn delete_old_voice_sessions(pool: &PgPool, days: i32) -> Result<u64, sqlx::Error> {
    sqlx::query(DELETE_OLD_VOICE_SESSIONS_SQL)
        .bind(days)
        .execute(pool)
        .await
        .map(|result| result.rows_affected())
}

/// Garde ANTI-PURGE-TOTALE : une retention <= 0 rend `NOW() - interval '0 day'`
/// egal a `NOW()` -> `WHERE created_at < NOW()` supprimerait TOUTE la table (et
/// une valeur negative supprimerait meme les lignes futures). On refuse alors
/// d'executer le DELETE : mieux vaut conserver les donnees qu'une purge totale
/// declenchee par une simple case de config erronee.
fn valid_retention(days: i64, label: &str) -> Option<i32> {
    let Some(valid) =
        platform_core::sentinel::domain::services::system::scheduling::valid_retention(days)
    else {
        warn!(
            days,
            table = label,
            "retention <= 0 -> DELETE ignore (garde anti purge totale)"
        );
        return None;
    };

    match i32::try_from(valid) {
        Ok(days) => Some(days),
        Err(_) => {
            warn!(
                days,
                table = label,
                "retention trop grande pour PostgreSQL -> DELETE ignore"
            );
            None
        }
    }
}

pub async fn run(pool: &PgPool, config: &CleanupConfig) -> Result<(), String> {
    let mut errors = Vec::new();

    // ── Voice sessions ──
    let voice_deleted =
        match valid_retention(config.voice_sessions_retention_days, "voice_sessions") {
            Some(days) => match delete_old_voice_sessions(pool, days).await {
                Ok(rows) => {
                    record_cleanup_success("voice_sessions", rows);
                    rows
                }
                Err(e) => {
                    record_cleanup_error("voice_sessions");
                    warn!(error = %e, "Erreur suppression voice_sessions");
                    errors.push(format!("voice_sessions: {e}"));
                    0
                }
            },
            None => 0,
        };

    // ── Logs / audit_logs / user_activity_log (meme retention) ──
    let logs_days = valid_retention(config.logs_retention_days, "logs/audit/user_activity");

    let logs_deleted = match logs_days {
        Some(days) => match delete_old_logs(pool, days).await {
            Ok(rows) => {
                record_cleanup_success("logs", rows);
                rows
            }
            Err(e) => {
                record_cleanup_error("logs");
                warn!(error = %e, "Erreur suppression logs");
                errors.push(format!("logs: {e}"));
                0
            }
        },
        None => 0,
    };

    // ── Ticket messages from closed tickets ──
    let ticket_msgs_deleted =
        match valid_retention(config.closed_tickets_retention_days, "ticket_messages") {
            Some(days) => match sqlx::query(
                "DELETE FROM ticket_messages WHERE ticket_id IN (
            SELECT id FROM tickets WHERE status = 'closed'
            AND updated_at < NOW() - make_interval(days => $1::int)
        )",
            )
            .bind(days)
            .execute(pool)
            .await
            {
                Ok(r) => {
                    let rows = r.rows_affected();
                    record_cleanup_success("ticket_messages", rows);
                    rows
                }
                Err(e) => {
                    record_cleanup_error("ticket_messages");
                    warn!(error = %e, "Erreur suppression ticket_messages");
                    errors.push(format!("ticket_messages: {e}"));
                    0
                }
            },
            None => 0,
        };

    // ── Audit logs ──
    let audit_deleted = match logs_days {
        Some(days) => match sqlx::query(
            "DELETE FROM ONLY audit_logs WHERE created_at < NOW() - make_interval(days => $1::int)",
        )
        .bind(days)
        .execute(pool)
        .await
        {
            Ok(r) => {
                let rows = r.rows_affected();
                record_cleanup_success("audit_logs", rows);
                rows
            }
            Err(e) => {
                record_cleanup_error("audit_logs");
                warn!(error = %e, "Erreur suppression audit_logs");
                errors.push(format!("audit_logs: {e}"));
                0
            }
        },
        None => 0,
    };

    // ── User activity log ──
    let activity_deleted = match logs_days {
        Some(days) => match sqlx::query(
            "DELETE FROM ONLY user_activity_log WHERE created_at < NOW() - make_interval(days => $1::int)",
        )
        .bind(days)
        .execute(pool)
        .await
        {
            Ok(r) => {
                let rows = r.rows_affected();
                record_cleanup_success("user_activity_log", rows);
                rows
            }
            Err(e) => {
                record_cleanup_error("user_activity_log");
                warn!(error = %e, "Erreur suppression user_activity_log");
                errors.push(format!("user_activity_log: {e}"));
                0
            }
        },
        None => 0,
    };

    info!(
        voice_sessions = voice_deleted,
        logs = logs_deleted,
        ticket_messages = ticket_msgs_deleted,
        audit_logs = audit_deleted,
        user_activity_log = activity_deleted,
        "Cleaned {} voice_sessions, {} logs, {} ticket_messages, {} audit_logs, {} user_activity_log",
        voice_deleted,
        logs_deleted,
        ticket_msgs_deleted,
        audit_deleted,
        activity_deleted,
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Erreurs partielles: {}", errors.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "./migrations/sentinel")]
    async fn deletes_only_logs_older_than_retention(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO logs (\"timestamp\", message) VALUES \
             (NOW() - INTERVAL '31 days', 'old cleanup test'), \
             (NOW() - INTERVAL '29 days', 'recent cleanup test')",
        )
        .execute(&pool)
        .await?;

        let deleted = delete_old_logs(&pool, 30).await?;
        let remaining: Vec<String> = sqlx::query_scalar(
            "SELECT message FROM logs WHERE message IN ('old cleanup test', 'recent cleanup test')",
        )
        .fetch_all(&pool)
        .await?;

        assert_eq!(deleted, 1);
        assert_eq!(remaining, vec!["recent cleanup test"]);
        Ok(())
    }

    /// Regression : le DELETE visait `created_at`, colonne inexistante sur
    /// `voice_sessions`. L'erreur etait avalee par le `warn!` du job, donc rien
    /// n'etait jamais purge sans que rien ne le signale. Ce test echoue si la
    /// colonne redevient fausse, puisque sqlx remonte alors l'erreur.
    #[sqlx::test(migrations = "./migrations/sentinel")]
    async fn deletes_only_voice_sessions_older_than_retention(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO voice_sessions \
             (guild_id, user_id, username, channel_id, started_at, ended_at) VALUES \
             ('1', '1', 'ancienne', '1', NOW() - INTERVAL '91 days', NOW() - INTERVAL '91 days'), \
             ('1', '2', 'recente', '1', NOW() - INTERVAL '89 days', NOW() - INTERVAL '89 days')",
        )
        .execute(&pool)
        .await?;

        let deleted = delete_old_voice_sessions(&pool, 90).await?;
        let remaining: Vec<String> =
            sqlx::query_scalar("SELECT username FROM voice_sessions ORDER BY username")
                .fetch_all(&pool)
                .await?;

        assert_eq!(deleted, 1);
        assert_eq!(remaining, vec!["recente"]);
        Ok(())
    }
}
