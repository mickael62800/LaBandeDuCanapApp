use sqlx::PgPool;
use tracing::{debug, info};

/// Supprime les bans vocaux expirés
pub async fn run(pool: &PgPool) -> Result<(), String> {
    let result = sqlx::query(
        "DELETE FROM voice_channel_bans WHERE expires_at IS NOT NULL AND expires_at <= NOW()",
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Cleanup bans: {e}"))?;

    let count = result.rows_affected();
    if count > 0 {
        info!(count, "Bans vocaux expirés nettoyés");
    } else {
        debug!("Aucun ban vocal expiré");
    }

    Ok(())
}
