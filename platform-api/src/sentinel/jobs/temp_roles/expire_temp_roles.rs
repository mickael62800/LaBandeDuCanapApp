use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct ExpiredRole {
    id: Uuid,
    guild_id: String,
    user_id: String,
    role_id: String,
}

/// Phase 4 B — Scan + emission Redis des roles temporaires expires.
///
/// Le worker ne peut PAS appeler `member.remove_role()` directement (pas de
/// connexion gateway Discord). Il emet un event via XADD sur la stream
/// `sentinel:events` (Phase 5B) que le `community-bot` consomme pour executer
/// le retrait Discord local + DELETE de la ligne en DB.
///
/// Pour eviter les doublons, on peut soit :
///   - laisser le bot DELETE la ligne apres remove_role reussi (pattern actuel)
///   - SUPPRIMER ici et le bot ne touche que Discord
///
/// On garde l'ancien pattern (le bot DELETE) pour rester compatible avec les
/// flows existants. Le worker se contente de PUBLIER l'event.
pub async fn run(pool: &PgPool, redis: &redis::aio::ConnectionManager) -> Result<(), String> {
    let expired: Vec<ExpiredRole> = sqlx::query_as::<_, ExpiredRole>(
        "SELECT id, guild_id, user_id, role_id FROM temp_roles \
         WHERE expires_at <= NOW() \
         ORDER BY expires_at ASC \
         LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query expired temp_roles: {e}"))?;

    if expired.is_empty() {
        debug!("Aucun role temporaire expire");
        return Ok(());
    }

    let mut conn = redis.clone();

    let mut published = 0u32;
    for role in &expired {
        // `temp_roles` est une infrastructure partagee (roles communautaires,
        // sanctions, sursis). Une expiration deja enregistree doit toujours
        // etre executee : la bloquer sur un toggle de module laisserait un
        // membre mute indefiniment.
        let payload = serde_json::json!({
            "event": "temp_role_expire",
            "data": {
                "guild_id": role.guild_id,
                "user_id": role.user_id,
                "role_id": role.role_id,
            }
        });
        let serialized = match serde_json::to_string(&payload) {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "serialize event");
                continue;
            }
        };

        let res = crate::sentinel::jobs::support::publish_event(&mut conn, &serialized).await;
        match res {
            Ok(_) => published += 1,
            Err(e) => warn!(role_id = %role.id, error = %e, "XADD failed"),
        }
    }

    if published > 0 {
        info!(
            published,
            total = expired.len(),
            "Roles temporaires expires : events emis vers community-bot"
        );
    }

    Ok(())
}
