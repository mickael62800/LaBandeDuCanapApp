//! Cloture des votes automod arrives a echeance.
//!
//! Quand `vote_enabled` est actif, une detection automod ouvre un vote des
//! moderateurs avec une echeance (`automod_reviews.voting_deadline`,
//! statut 'voting'). Ce job periodique :
//!   1. trouve les reviews 'voting' dont l'echeance est depassee,
//!   2. lit le quorum + la regle de tie-break dans `bot_guild_config`,
//!   3. appelle l'API `POST /api/automod/reviews/{id}/decide` qui depouille
//!      (logique de tally cote core) et passe la review en 'decided',
//!      puis broadcast l'event `automod_review_decided`.
//!
//! Le bot consomme l'event pour editer la carte (verdict) et reveler le
//! bouton admin de finalisation. Idempotence : `decide` cote API rejette
//! une review qui n'est plus en 'voting' (Conflict ignore ici).

use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use platform_common_worker::api;

use super::DEFAULT_VOTE_QUORUM;

#[derive(Debug, sqlx::FromRow)]
struct ExpiredReview {
    id: Uuid,
    quorum: i32,
    tie_action: String,
}

pub async fn run(pool: &PgPool) -> Result<(), String> {
    // Quorum/tie lus par guild depuis bot_guild_config (defauts si absents).
    let rows: Vec<ExpiredReview> = sqlx::query_as::<_, ExpiredReview>(
        r#"SELECT r.id,
                  COALESCE(
                      CASE WHEN q.config_value ~ '^\d+$' THEN q.config_value::int ELSE NULL END,
                      $1
                  ) AS quorum,
                  COALESCE(t.config_value, 'ignore') AS tie_action
             FROM automod_reviews r
             LEFT JOIN bot_guild_config q
                 ON q.guild_id = r.guild_id AND q.bot_name = 'automod-bot'
                AND q.config_key = 'vote_quorum'
             LEFT JOIN bot_guild_config t
                 ON t.guild_id = r.guild_id AND t.bot_name = 'automod-bot'
                AND t.config_key = 'vote_tie_action'
            WHERE r.status = 'voting'
              AND r.voting_deadline IS NOT NULL
              AND r.voting_deadline < NOW()
            ORDER BY r.voting_deadline ASC
            LIMIT 50"#,
    )
    .bind(DEFAULT_VOTE_QUORUM)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query expired votes: {e}"))?;

    if rows.is_empty() {
        return Ok(());
    }

    let mut closed = 0u32;
    for r in &rows {
        let body = serde_json::json!({ "quorum": r.quorum, "tie_action": r.tie_action });
        match api::post_json::<_, serde_json::Value>(
            &format!("/api/automod/reviews/{}/decide", r.id),
            &body,
        )
        .await
        {
            Ok(_) => closed += 1,
            // Conflict (deja cloture par un autre replica) ou autre : on log
            // sans faire echouer tout le batch.
            Err(e) => warn!(review_id = %r.id, error = %e, "echec cloture vote automod"),
        }
    }

    info!(expired = rows.len(), closed, "Votes automod clotures");
    Ok(())
}

