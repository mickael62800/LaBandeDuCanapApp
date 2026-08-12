//! Auto-backup periodique : publie `guild_backup:capture_requested` pour les
//! guilds dont l'intervalle configure est ecoule depuis la derniere capture.
//!
//! Cadence worker = frequence de VERIFICATION (defaut 30 min). L'intervalle
//! FIN (24h par defaut) est lu par guild dans `bot_guild_config`
//! (`auto_backup_interval_hours`). Un tick ne publie que les guilds "due".
//!
//! Anti-double-publication : la capture est asynchrone (event -> bot -> nouveau
//! snapshot en quelques secondes). Pour eviter de re-publier au tick suivant
//! avant que le snapshot n'existe, on pose un GARDE Redis
//! `guild_backup:auto:pending:{guild}` en `SET NX EX <ttl>` (defaut 10 min) au
//! moment du publish. Choix du garde Redis TTL (le plus leger) plutot qu'une
//! table d'etat : pas de migration, auto-expiration, coherent avec les autres
//! workers qui ne persistent pas d'etat de publication.
//!
//! Enveloppe publiee (IDENTIQUE a l'API, cf. broadcaster.rs + le handler
//! request_capture) :
//! {"event":"guild_backup:capture_requested","guild_id":"<id>",
//!  "data":{"guild_id":"<id>","label":"Auto-backup ...","requested_by":"auto",
//!          "sig":"<hmac-sha256 hex>"}}
//!
//! Le `sig` n'est pas optionnel : le bot rejette tout event `guild_backup:*` non
//! signe, parce que `sentinel:events` est le bus COMMUN aux trois plateformes et
//! que la restauration y detruit un serveur entier. Ce worker signe avec le meme
//! secret que l'API (`SENTINEL_API_KEY`).

use std::collections::HashMap;

use sentinel_core::domain::entities::system::config_parsers::parse_bool_str;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{debug, info, warn};

use platform_common_worker::redis_helpers;

const BOT_NAME: &str = "guild-backup-bot";
const EVENT_CAPTURE: &str = "guild_backup:capture_requested";
const DEFAULT_INTERVAL_HOURS: i64 = 24;
/// TTL du garde anti-double-publication (secondes). Doit couvrir largement le
/// temps entre publish et apparition du nouveau snapshot.
const PENDING_GUARD_TTL_SECS: usize = 600;

/// Etat de config auto-backup d'une guild.
struct GuildAuto {
    enabled: bool,
    interval_hours: i64,
}

pub async fn run(pool: &PgPool, redis: &redis::aio::ConnectionManager) -> Result<(), String> {
    // 1. Config auto-backup par guild (enabled + interval_hours) en 1 query.
    let configs = load_configs(pool).await;
    if configs.is_empty() {
        debug!("guild_backup auto: aucune guild configuree");
        return Ok(());
    }

    // 2. Derniere sauvegarde par guild.
    let last_by_guild = load_last_snapshots(pool).await?;

    let now = Utc::now();
    let mut conn = redis.clone();
    // Lu une fois par tick plutot que par guilde : c'est la meme valeur, et un
    // `env::var` par iteration n'apporte rien.
    let secret = std::env::var("SENTINEL_API_KEY").unwrap_or_default();

    let mut published = 0u32;
    for (guild_id, cfg) in &configs {
        if !cfg.enabled {
            continue;
        }
        let last = last_by_guild.get(guild_id).copied();
        if !is_due(last, cfg.interval_hours, now) {
            continue;
        }

        // Garde anti-double-publication : SET NX EX. Si la cle existe deja
        // (capture recente encore en vol), on saute ce tick.
        let key = format!("guild_backup:auto:pending:{guild_id}");
        let set: Option<String> = redis::cmd("SET")
            .arg(&key)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(PENDING_GUARD_TTL_SECS)
            .query_async(&mut conn)
            .await
            .map_err(|e| format!("redis SET NX guard: {e}"))?;
        if set.is_none() {
            debug!(guild = %guild_id, "guild_backup auto: capture deja en attente, skip");
            continue;
        }

        let label = format!("Auto-backup {}", now.format("%Y-%m-%d %H:%M"));
        let payload = serde_json::json!({
            "event": EVENT_CAPTURE,
            "guild_id": guild_id,
            "data": {
                "guild_id": guild_id,
                "label": label,
                "requested_by": "auto",
                // Le bot rejette un event `guild_backup:*` non signe. Ce worker
                // est un producteur legitime au meme titre que l'API : il porte
                // le meme secret, il signe le meme message canonique.
                "sig": sign_capture(&secret, guild_id),
            }
        });

        match redis_helpers::xadd_event(&mut conn, &payload.to_string()).await {
            Ok(_) => published += 1,
            Err(e) => {
                warn!(guild = %guild_id, error = %e, "guild_backup auto: XADD capture_requested echoue");
            }
        }
    }

    if published > 0 {
        info!(published, "guild_backup auto: capture_requested publies");
    }
    Ok(())
}

/// Signature HMAC-SHA256 de `guild_backup:capture_requested`.
///
/// Message canonique reproduit a l'identique dans
/// `sentinel-api/.../http/event_signing.rs` et `sentinel-bot/src/shared/event_signing.rs`.
/// Le troisieme exemplaire est ici parce que ce worker est le troisieme
/// producteur/consommateur du contrat, et qu'aucun des trois crates ne peut
/// dependre des deux autres. Modifier le format sans le repercuter partout fait
/// rejeter l'event — silencieux du point de vue du worker, mais visible dans les
/// logs du bot, et surtout sans destruction.
///
/// Secret vide (dev) -> signature vide, que le bot n'exige alors pas.
fn sign_capture(secret: &str, guild_id: &str) -> String {
    if secret.is_empty() {
        return String::new();
    }
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let message = format!("guild_backup:capture:{guild_id}");
    let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes()).expect("cle HMAC");
    mac.update(message.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Vrai si une sauvegarde est due : jamais sauvegardee, ou l'intervalle est
/// ecoule depuis la derniere capture. Un intervalle <= 0 est traite comme le
/// defaut (garde-fou contre une valeur absurde en config).
fn is_due(last: Option<DateTime<Utc>>, interval_hours: i64, now: DateTime<Utc>) -> bool {
    sentinel_core::domain::services::system::scheduling::is_due(
        last,
        interval_hours,
        DEFAULT_INTERVAL_HOURS,
        now,
    )
}

/// Charge `auto_backup_enabled` + `auto_backup_interval_hours` par guild.
async fn load_configs(pool: &PgPool) -> HashMap<String, GuildAuto> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT guild_id, config_key, config_value FROM bot_guild_config \
         WHERE bot_name = $1 \
           AND config_key IN ('auto_backup_enabled', 'auto_backup_interval_hours')",
    )
    .bind(BOT_NAME)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut map: HashMap<String, GuildAuto> = HashMap::new();
    for (guild_id, key, value) in rows {
        let entry = map.entry(guild_id).or_insert(GuildAuto {
            enabled: false,
            interval_hours: DEFAULT_INTERVAL_HOURS,
        });
        match key.as_str() {
            "auto_backup_enabled" => entry.enabled = parse_bool_str(&value),
            "auto_backup_interval_hours" => {
                if let Ok(h) = value.parse::<i64>() {
                    entry.interval_hours = h;
                }
            }
            _ => {}
        }
    }
    map
}

/// Derniere `created_at` par guild dans `guild_snapshots`.
async fn load_last_snapshots(pool: &PgPool) -> Result<HashMap<String, DateTime<Utc>>, String> {
    let rows: Vec<(String, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT guild_id, MAX(created_at) AS last FROM guild_snapshots GROUP BY guild_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query last snapshots: {e}"))?;

    Ok(rows
        .into_iter()
        .filter_map(|(g, last)| last.map(|t| (g, t)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn t(h: i64) -> DateTime<Utc> {
        Utc::now() - Duration::hours(h)
    }

    #[test]
    fn due_when_never_backed_up() {
        assert!(is_due(None, 24, Utc::now()));
    }

    #[test]
    fn due_when_interval_elapsed() {
        // Derniere capture il y a 25h, intervalle 24h -> due.
        assert!(is_due(Some(t(25)), 24, Utc::now()));
    }

    #[test]
    fn not_due_when_recent() {
        // Derniere capture il y a 1h, intervalle 24h -> pas due.
        assert!(!is_due(Some(t(1)), 24, Utc::now()));
    }

    #[test]
    fn due_exactly_at_interval() {
        // Bord : exactement l'intervalle ecoule -> due (>=).
        let now = Utc::now();
        let last = now - Duration::hours(24);
        assert!(is_due(Some(last), 24, now));
    }

    #[test]
    fn invalid_interval_falls_back_to_default() {
        // interval <= 0 -> defaut 24h. Il y a 25h -> due, il y a 1h -> pas due.
        assert!(is_due(Some(t(25)), 0, Utc::now()));
        assert!(!is_due(Some(t(1)), -5, Utc::now()));
    }
}
