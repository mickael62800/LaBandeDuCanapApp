//! Delai d'acceptation du reglement : relance puis expulsion des arrivants qui
//! n'ont pas clique.
//!
//! Ne concerne PAS les comptes suspects, qui relevent de la quarantaine
//! (`security::kick_expired_quarantine`). Ici, la population est celle des
//! arrivants ORDINAIRES, et le rythme se compte en jours, pas en secondes.
//!
//! Le job ne parle pas a Discord : il publie des evenements que le bot
//! consomme. L'API ne connait ni les salons ni les messages prives.
//!
//! La decision — relancer, expulser, ou laisser attendre — vit dans le domaine
//! (`community::rules_deadline::decide`). Ici on ne fait que l'appliquer.

use std::collections::HashMap;

use sqlx::PgPool;
use tracing::{debug, info, warn};

use platform_core::sentinel::domain::entities::community::rules_deadline::{
    decide, PendingRulesDeadline, RulesDeadlineAction, RulesDeadlineSettings,
};

/// Nombre d'echeances traitees par passage. Bornee pour qu'un serveur qui
/// active le delai sur une file de plusieurs milliers de membres ne tente pas
/// de tous les expulser dans le meme tour.
const LOT: i64 = 100;

/// Reglages d'une guilde, lus une seule fois par passage.
async fn reglages(pool: &PgPool, guild_id: &str) -> RulesDeadlineSettings {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT config_key, config_value FROM bot_guild_config \
         WHERE guild_id = $1 AND bot_name = 'welcome-bot' \
           AND config_key IN ('rules_deadline_enabled', 'rules_deadline_secs', \
                              'rules_reminder_secs', 'rules_kick_enabled')",
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let map: HashMap<String, String> = rows.into_iter().collect();
    let bool_de = |cle: &str, defaut: bool| {
        map.get(cle)
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "true" | "1" | "yes" | "on"
                )
            })
            .unwrap_or(defaut)
    };
    let i64_de = |cle: &str, defaut: i64| {
        map.get(cle)
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(defaut)
    };

    let defauts = RulesDeadlineSettings::default();
    RulesDeadlineSettings {
        // Fail closed : sans reglage explicite, le systeme dort.
        enabled: bool_de("rules_deadline_enabled", false),
        deadline_secs: i64_de("rules_deadline_secs", defauts.deadline_secs),
        reminder_secs: i64_de("rules_reminder_secs", defauts.reminder_secs),
        kick_enabled: bool_de("rules_kick_enabled", defauts.kick_enabled),
    }
    .sanitized()
}

pub async fn run(
    pool: &PgPool,
    redis: &redis::aio::ConnectionManager,
) -> Result<(usize, usize), String> {
    let mut conn = redis.clone();
    let mut relances = 0usize;
    let mut expulsions = 0usize;

    // Un cache local au passage : la meme guilde revient autant de fois qu'elle
    // a de membres en attente, et relire sa configuration pour chacun ferait
    // autant d'allers-retours SQL inutiles. Local, pour qu'un changement de
    // reglage soit vu au tour suivant.
    let mut cache: HashMap<String, RulesDeadlineSettings> = HashMap::new();

    // ── Expulsions ──
    //
    // Traitees en premier : une relance due en meme temps que l'echeance
    // n'aurait aucun interet, et le domaine l'exprime deja dans le meme ordre.
    let expirees: Vec<(
        String,
        String,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        "SELECT guild_id, user_id, expires_at, reminded_at \
             FROM welcome_rules_pending WHERE expires_at <= NOW() \
             ORDER BY expires_at ASC LIMIT $1",
    )
    .bind(LOT)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("list rules expired: {e}"))?;

    for (guild_id, user_id, expires_at, reminded_at) in expirees {
        if !crate::sentinel::jobs::support::is_enabled(pool, &guild_id, "welcome-bot").await {
            continue;
        }
        let cfg = match cache.get(&guild_id) {
            Some(c) => c.clone(),
            None => {
                let c = reglages(pool, &guild_id).await;
                cache.insert(guild_id.clone(), c.clone());
                c
            }
        };
        let attente = PendingRulesDeadline {
            guild_id: guild_id.clone(),
            user_id: user_id.clone(),
            expires_at,
            reminded_at,
        };

        match decide(&cfg, &attente, chrono::Utc::now()) {
            RulesDeadlineAction::Kick => {}
            // Expulsion desactivee, ou systeme eteint : la ligne RESTE. La
            // supprimer effacerait la file d'attente que l'administrateur a
            // choisi de laisser grandir, et la reactivation partirait de rien.
            _ => continue,
        }

        // Claim atomique : la garde sur `expires_at` fait qu'une acceptation
        // survenue entre-temps — le bot supprime alors la ligne — annule
        // l'expulsion au lieu de la doubler.
        let supprime = sqlx::query(
            "DELETE FROM welcome_rules_pending \
             WHERE guild_id = $1 AND user_id = $2 AND expires_at <= NOW()",
        )
        .bind(&guild_id)
        .bind(&user_id)
        .execute(pool)
        .await
        .map_err(|e| format!("claim rules expired: {e}"))?;
        if supprime.rows_affected() == 0 {
            continue;
        }

        let payload = serde_json::json!({
            "event": "welcome_rules_expired",
            "data": { "guild_id": guild_id, "user_id": user_id }
        });
        if let Err(e) =
            crate::sentinel::jobs::support::publish_event_json(&mut conn, &payload).await
        {
            warn!(error = %e, guild = %guild_id, user = %user_id, "publication welcome_rules_expired echouee");
        }
        expulsions += 1;
    }

    // ── Relances ──
    let a_relancer: Vec<(
        String,
        String,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        "SELECT guild_id, user_id, expires_at, reminded_at \
             FROM welcome_rules_pending \
             WHERE reminded_at IS NULL AND expires_at > NOW() \
             ORDER BY expires_at ASC LIMIT $1",
    )
    .bind(LOT)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("list rules reminder due: {e}"))?;

    for (guild_id, user_id, expires_at, reminded_at) in a_relancer {
        if !crate::sentinel::jobs::support::is_enabled(pool, &guild_id, "welcome-bot").await {
            continue;
        }
        let cfg = match cache.get(&guild_id) {
            Some(c) => c.clone(),
            None => {
                let c = reglages(pool, &guild_id).await;
                cache.insert(guild_id.clone(), c.clone());
                c
            }
        };
        let attente = PendingRulesDeadline {
            guild_id: guild_id.clone(),
            user_id: user_id.clone(),
            expires_at,
            reminded_at,
        };

        if decide(&cfg, &attente, chrono::Utc::now()) != RulesDeadlineAction::Remind {
            continue;
        }

        // `reminded_at` est pose AVANT la publication, sous garde `IS NULL` :
        // un balayage regulier n'envoie pas un message prive a chaque passage.
        // En cas d'echec de publication, la relance est perdue plutot que
        // doublee — un membre prevenu deux fois est plus penible qu'un membre
        // non prevenu, qui sera de toute facon expulse avec un preavis visible.
        let reclame = sqlx::query(
            "UPDATE welcome_rules_pending SET reminded_at = NOW() \
             WHERE guild_id = $1 AND user_id = $2 AND reminded_at IS NULL",
        )
        .bind(&guild_id)
        .bind(&user_id)
        .execute(pool)
        .await
        .map_err(|e| format!("claim rules reminder: {e}"))?;
        if reclame.rows_affected() == 0 {
            continue;
        }

        let payload = serde_json::json!({
            "event": "welcome_rules_reminder",
            "data": {
                "guild_id": guild_id,
                "user_id": user_id,
                "expires_at": expires_at.to_rfc3339(),
            }
        });
        if let Err(e) =
            crate::sentinel::jobs::support::publish_event_json(&mut conn, &payload).await
        {
            warn!(error = %e, guild = %guild_id, user = %user_id, "publication welcome_rules_reminder echouee");
        }
        relances += 1;
    }

    if relances > 0 || expulsions > 0 {
        info!(relances, expulsions, "Delai d'acceptation du reglement");
    } else {
        debug!("Aucune echeance de reglement a traiter");
    }
    Ok((relances, expulsions))
}
