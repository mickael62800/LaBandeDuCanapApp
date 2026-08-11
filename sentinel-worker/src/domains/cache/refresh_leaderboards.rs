use sqlx::PgPool;
use tracing::{info, warn};

/// Phase 2 A.2 — Refresh CONCURRENT des vues materialisees leaderboards.
///
/// `REFRESH MATERIALIZED VIEW CONCURRENTLY` ne pose qu'un verrou ROW EXCLUSIVE
/// (pas ACCESS EXCLUSIVE), ce qui permet aux lectures de continuer pendant
/// le refresh. Necessite l'index UNIQUE cree dans la migration 102.
///
/// Cible toutes les MV de leaderboard une par une. Si une echoue, on log
/// et on continue les autres (best-effort).
pub async fn run(pool: &PgPool) -> Result<(), String> {
    const VIEWS: &[&str] = &["mv_level_leaderboard"];

    let mut refreshed = 0u32;
    for view in VIEWS {
        // PostgreSQL refuse CONCURRENTLY tant que la vue creee WITH NO DATA
        // n'a jamais ete peuplee. Le premier passage doit donc etre bloquant ;
        // les suivants retrouvent le mode concurrent, sans bloquer les reads.
        let populated = match sqlx::query_scalar::<_, bool>(
            "SELECT c.relispopulated \
             FROM pg_catalog.pg_class c \
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
             WHERE n.nspname = 'public' AND c.relname = $1 AND c.relkind = 'm'",
        )
        .bind(view)
        .fetch_optional(pool)
        .await
        {
            Ok(Some(populated)) => populated,
            Ok(None) => {
                warn!(view, "Materialized leaderboard view not found");
                continue;
            }
            Err(e) => {
                warn!(view, error = %e, "Unable to inspect materialized leaderboard view");
                continue;
            }
        };

        let concurrently = if populated { "CONCURRENTLY " } else { "" };
        let sql = format!("REFRESH MATERIALIZED VIEW {concurrently}{view}");
        match sqlx::query(&sql).execute(pool).await {
            Ok(_) => refreshed += 1,
            Err(e) => warn!(view, error = %e, "REFRESH MATERIALIZED VIEW failed"),
        }
    }

    if refreshed > 0 {
        info!(
            refreshed,
            total = VIEWS.len(),
            "Leaderboards materialized views refreshed"
        );
    }

    Ok(())
}
