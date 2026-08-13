use sqlx::PgPool;
use tracing::{info, warn};

/// Phase 2 A.2 — Synchronise la table `user_cache` (source de verite des
/// usernames Discord) en crawlant les tables hot ou les usernames sont
/// denormalises.
///
/// Approche pragmatique : plutot que de creer un listener Discord dedie,
/// on agrege les `(guild_id, user_id, username)` les plus recents depuis
/// les tables hot et on upsert dans `user_cache`. Le `DISTINCT ON` garde
/// la ligne au `updated_at` le plus recent par couple (guild, user).
///
/// Cout : un seul query unifiee + un upsert batch via `INSERT ... SELECT`,
/// donc pas de boucle row-par-row. Tourne idealement toutes les 15 min.
pub async fn run(pool: &PgPool) -> Result<(), String> {
    let result = sqlx::query(
        r#"
        INSERT INTO user_cache (guild_id, user_id, username, updated_at)
        SELECT DISTINCT ON (guild_id, user_id)
            guild_id, user_id, username, updated_at
        FROM (
            SELECT guild_id, user_id, username, updated_at FROM user_levels
            WHERE username IS NOT NULL AND username <> ''
            UNION ALL
            SELECT guild_id, user_id, username, updated_at FROM user_stats
            WHERE username IS NOT NULL AND username <> ''
        ) AS sources
        ORDER BY guild_id, user_id, updated_at DESC
        ON CONFLICT (guild_id, user_id) DO UPDATE
        SET username = EXCLUDED.username,
            updated_at = EXCLUDED.updated_at
        WHERE user_cache.updated_at < EXCLUDED.updated_at
        "#,
    )
    .execute(pool)
    .await;

    match result {
        Ok(res) => {
            info!(rows = res.rows_affected(), "user_cache synchronise");
            Ok(())
        }
        Err(e) => {
            warn!(error = %e, "Echec sync_user_cache");
            Err(e.to_string())
        }
    }
}
