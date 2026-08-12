//! Phase 5B — Live tail Redis Streams pour le relay WebSocket.
//!
//! Le gateway lit en XREAD `$` la stream `sentinel:events` sans consumer group.
//! Semantique fire-and-forget preservee (identique a l'ancien pub/sub) :
//! - Si le gateway est down, les events ne sont PAS rejoues au redemarrage
//!   (on demarre au "dernier ID" au moment de la reconnexion).
//! - Si un client WS est deconnecte, ses events sont perdus cote client.
//!
//! Cette semantique est volontaire pour un dashboard temps reel : pas de
//! rattrapage de 1000 events obsoletes au redemarrage, juste la suite.
//!
//! Pour les consumers durables (moderation-bot, ticket-bot), voir
//! `crate::shared::event_bus::listen_stream_group` qui utilise XREADGROUP + XACK.

use std::sync::Arc;

use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::AsyncCommands;
use tracing::{error, info, warn};

use crate::broadcaster::{EventBroadcaster, WsEvent};
use crate::logger::GatewayLogger;

const PAYLOAD_FIELD: &str = "payload";
const BLOCK_MS: u64 = 5_000;
const BATCH_COUNT: usize = 64;

/// Lance le tail Redis Streams avec reconnexion automatique et exponential backoff.
pub async fn run_redis_subscriber(
    redis_url: &str,
    stream_key: &str,
    broadcaster: Arc<EventBroadcaster>,
    logger: Arc<GatewayLogger>,
    base_delay_secs: u64,
    max_delay_secs: u64,
) {
    let mut delay = base_delay_secs;

    loop {
        match tail_loop(redis_url, stream_key, &broadcaster, &logger).await {
            Ok(()) => {
                warn!("Redis stream tail disconnected, reconnecting in {delay}s...");
                logger.warn(
                    "Redis stream tail deconnecte, reconnexion...",
                    serde_json::json!({
                        "event_type": "redis.disconnected",
                        "retry_delay_secs": delay,
                    }),
                );
            }
            Err(e) => {
                error!(error = %e, delay_secs = delay, "Redis stream tail error, reconnecting...");
                logger.error(
                    "Erreur Redis stream tail",
                    serde_json::json!({
                        "event_type": "redis.error",
                        "error": e.to_string(),
                        "retry_delay_secs": delay,
                    }),
                );
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;

        // Exponential backoff: double le delay a chaque echec, jusqu'au max
        delay = (delay * 2).min(max_delay_secs);
    }
}

async fn tail_loop(
    redis_url: &str,
    stream_key: &str,
    broadcaster: &EventBroadcaster,
    logger: &GatewayLogger,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_multiplexed_async_connection().await?;

    info!(stream = %stream_key, "Redis stream tail connecte");
    logger.info(
        "Redis stream tail connecte",
        serde_json::json!({
            "event_type": "redis.connected",
            "stream": stream_key,
        }),
    );

    // Demarrer au dernier ID : on ignore tout ce qui s'est accumule avant la connexion.
    // Cela preserve la semantique fire-and-forget de l'ancien pub/sub.
    let mut last_id = String::from("$");

    let opts = StreamReadOptions::default()
        .block(BLOCK_MS as usize)
        .count(BATCH_COUNT);

    // Stats agregees pour eviter de spammer 1 log par event Discord
    let mut events_broadcast: u64 = 0;
    let mut payloads_invalid: u64 = 0;
    let mut payloads_missing: u64 = 0;
    let mut last_stats_log = std::time::Instant::now();
    let stats_interval = std::time::Duration::from_secs(60);

    loop {
        let reply: Option<StreamReadReply> = conn
            .xread_options(&[stream_key], &[last_id.as_str()], &opts)
            .await?;

        // Flush stats toutes les 60s — un log "alive" pour montrer que ca tourne
        // + visibilite sur le throughput et les erreurs accumulees.
        if last_stats_log.elapsed() >= stats_interval {
            if events_broadcast + payloads_invalid + payloads_missing > 0 {
                logger.info(
                    "Stats gateway (60s)",
                    serde_json::json!({
                        "event_type": "gateway.stats",
                        "events_broadcast": events_broadcast,
                        "payloads_invalid": payloads_invalid,
                        "payloads_missing": payloads_missing,
                        "clients_connected": broadcaster.connected_count(),
                    }),
                );
            }
            events_broadcast = 0;
            payloads_invalid = 0;
            payloads_missing = 0;
            last_stats_log = std::time::Instant::now();
        }

        let Some(reply) = reply else { continue };

        for key in reply.keys {
            for entry in key.ids {
                // Extraire le champ `payload` qui contient le JSON de l'event
                let payload_str = match entry.map.get(PAYLOAD_FIELD) {
                    Some(redis::Value::BulkString(bytes)) => {
                        String::from_utf8_lossy(bytes).into_owned()
                    }
                    Some(redis::Value::SimpleString(s)) => s.clone(),
                    _ => {
                        warn!(entry_id = %entry.id, "Entry sans champ payload, ignoree");
                        payloads_missing += 1;
                        last_id = entry.id.clone();
                        continue;
                    }
                };

                match serde_json::from_str::<WsEvent>(&payload_str) {
                    Ok(event) => {
                        broadcaster.broadcast(event);
                        events_broadcast += 1;
                    }
                    Err(e) => {
                        warn!(error = %e, "Event stream invalide, ignore");
                        payloads_invalid += 1;
                        // 1er payload invalide -> log API (sinon flooding)
                        if payloads_invalid == 1 {
                            logger.warn(
                                "Event Redis stream invalide",
                                // Pas d'extrait du payload : `sentinel:events`
                                // transporte du contenu de message et des
                                // identifiants Discord, et ce log part dans le
                                // journal technique consultable en back-office.
                                // Un event malforme se diagnostique avec sa
                                // taille et l'erreur serde ; l'inspecter demande
                                // de lire la stream, ce qui est un geste
                                // deliberé et pas un effet de bord d'un log.
                                serde_json::json!({
                                    "event_type": "gateway.payload_invalid",
                                    "error": e.to_string(),
                                    "payload_bytes": payload_str.len(),
                                }),
                            );
                        }
                    }
                }

                last_id = entry.id.clone();
            }
        }
    }
}
