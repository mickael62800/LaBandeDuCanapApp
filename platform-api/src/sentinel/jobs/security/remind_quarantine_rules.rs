//! Rappel avant expulsion : previent en message prive les comptes SUSPECTS
//! qui n'ont pas encore passe la verification et dont l'echeance approche.
//!
//! Ne concerne PAS l'acceptation du reglement par un membre ordinaire : la
//! quarantaine n'est posee que sur suspicion (raid, compte trop jeune, alt).
//!
//! Le job ne parle pas a Discord — il publie `quarantine_rules_reminder`, que
//! le bot consomme pour envoyer le message prive. Meme decoupage que
//! `kick_expired_quarantine` : l'API ne connait pas les canaux Discord.
//!
//! IDEMPOTENCE. `reminded_at` est pose AVANT la publication, sous garde
//! `IS NULL` : deux instances du worker ne peuvent pas rappeler le meme membre,
//! et un balayage toutes les quinze secondes n'envoie pas un message toutes les
//! quinze secondes. En cas d'echec de publication, le rappel est perdu plutot
//! que double — un membre prevenu deux fois est plus penible qu'un membre non
//! prevenu, qui garde de toute facon son delai entier.

use sqlx::PgPool;
use tracing::{debug, info, warn};

#[derive(sqlx::FromRow)]
struct RappelDu {
    guild_id: String,
    user_id: String,
    /// Secondes restantes avant l'expulsion, au moment du balayage. C'est ce
    /// que le message annonce ; le recalculer cote bot donnerait une valeur
    /// differente de celle qui a declenche le rappel.
    secondes_restantes: f64,
}

pub async fn run(pool: &PgPool, redis: &redis::aio::ConnectionManager) -> Result<(), String> {
    // Le delai de rappel est un reglage PAR GUILDE : la jointure va le chercher
    // la ou il vit. Absent, on retombe sur une heure — le defaut du schema.
    //
    // Un rappel n'a de sens que si une expulsion est prevue : une guilde qui a
    // desactive l'expulsion automatique ne menace personne.
    // La valeur est saisie a la main dans le tableau de bord : une coquille
    // ('1h', 'abc') ferait echouer la conversion, donc la requete, donc le job
    // ENTIER — plus un seul rappel nulle part, sans rien dans les journaux qui
    // pointe vers la guilde fautive. Ce qui n'est pas une suite de chiffres
    // retombe donc sur le defaut.
    let candidats: Vec<RappelDu> = sqlx::query_as(
        "WITH reglage AS ( \
             SELECT q.guild_id, q.user_id, q.expires_at, \
                    CASE WHEN r.config_value ~ '^[0-9]+$' \
                         THEN r.config_value::double precision \
                         ELSE 3600 END AS rappel_secs, \
                    COALESCE(k.config_value, 'true') AS expulsion \
             FROM security_quarantine_pending q \
             LEFT JOIN bot_guild_config r \
               ON r.guild_id = q.guild_id AND r.bot_name = 'security-bot' \
              AND r.config_key = 'quarantine_reminder_secs' \
             LEFT JOIN bot_guild_config k \
               ON k.guild_id = q.guild_id AND k.bot_name = 'security-bot' \
              AND k.config_key = 'quarantine_kick_enabled' \
             WHERE q.reminded_at IS NULL AND q.expires_at > NOW() \
         ) \
         SELECT guild_id, user_id, \
                EXTRACT(EPOCH FROM (expires_at - NOW())) AS secondes_restantes \
         FROM reglage \
         WHERE expulsion IN ('true', '1') \
           AND rappel_secs > 0 \
           AND expires_at - NOW() <= make_interval(secs => rappel_secs) \
         ORDER BY expires_at ASC LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query rappels quarantaine: {e}"))?;

    if candidats.is_empty() {
        debug!("Aucun rappel de reglement a envoyer");
        return Ok(());
    }

    let mut conn = redis.clone();
    let mut envoyes = 0u32;

    for c in &candidats {
        if !crate::sentinel::jobs::support::is_enabled(pool, &c.guild_id, "security-bot").await {
            continue;
        }
        // Claim atomique : la garde `IS NULL` fait echouer la seconde
        // instance, qui passe alors au suivant.
        let pose = sqlx::query(
            "UPDATE security_quarantine_pending SET reminded_at = NOW() \
             WHERE guild_id = $1 AND user_id = $2 AND reminded_at IS NULL",
        )
        .bind(&c.guild_id)
        .bind(&c.user_id)
        .execute(pool)
        .await
        .map_err(|e| format!("claim rappel: {e}"))?;
        if pose.rows_affected() == 0 {
            continue;
        }

        let payload = serde_json::json!({
            "event": "quarantine_rules_reminder",
            "data": {
                "guild_id": c.guild_id,
                "user_id": c.user_id,
                "seconds_left": c.secondes_restantes.max(0.0) as i64,
            }
        });
        if let Err(e) =
            crate::sentinel::jobs::support::publish_event_json(&mut conn, &payload).await
        {
            warn!(error = %e, guild = %c.guild_id, "XADD quarantine_rules_reminder echoue");
            continue;
        }
        envoyes += 1;
    }

    if envoyes > 0 {
        info!(envoyes, "Rappels d'acceptation du reglement publies");
    }
    Ok(())
}
