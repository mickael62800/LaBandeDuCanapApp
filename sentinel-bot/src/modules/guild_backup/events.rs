//! Consumer Redis Streams — pilotage web de guild-backup.
//!
//! Ecoute `sentinel:events` (consumer group `guild-backup-bot`) et declenche
//! capture / restore / wipe en reponse aux events publies par l'API/le web :
//!
//! - `guild_backup:capture_requested` — data `{guild_id, label, requested_by, sig}`
//! - `guild_backup:restore_requested` — data `{guild_id, snapshot_id, wipe, requested_by, sig}`
//!
//! Les deux portent une **signature HMAC** verifiee avant toute action (cf.
//! [`crate::shared::event_signing`]) : `sentinel:events` est le bus commun aux
//! trois plateformes, et `restore` avec `wipe` supprime l'integralite des
//! salons, roles et emojis du serveur.
//!
//! Ces memes actions sont aussi declenchables par la slash-command `/backup`
//! (chemin interaction, inchange). Ici le feedback est HEADLESS (log tracing
//! via [`ProgressSink::Headless`]).
//!
//! Robustesse : toute erreur metier est loggee mais n'interrompt jamais le
//! consumer — l'event_bus ACK systematiquement pour ne pas boucler. On ne
//! traite un event que si le composant est `enabled()` pour la guild concernee.

use std::sync::Arc;

use serenity::all::{Context, GuildId};
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::event_bus;
use crate::shared::event_signing;
use crate::shared::heartbeat::ApiClientKey;

use super::guild_config::Config;
use super::progress::ProgressSink;
use super::{api_client, capture, restore, wipe};

/// Nom du consumer group (unique a ce composant).
const GROUP: &str = "guild-backup-bot";

const EVENT_CAPTURE: &str = "guild_backup:capture_requested";
const EVENT_RESTORE: &str = "guild_backup:restore_requested";

/// Spawn le consumer d'events dans le runtime du bot. Appele au `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = event_bus::default_consumer_name();
        event_bus::listen_stream_group(GROUP.to_string(), consumer, move |payload_json| {
            let ctx = ctx.clone();
            async move {
                // Best-effort total : jamais de panique/erreur remontee au bus.
                handle_event(&ctx, &payload_json).await;
            }
        })
        .await;
    });
}

async fn api(ctx: &Context) -> Option<Arc<BaseApiClient>> {
    ctx.data.read().await.get::<ApiClientKey>().cloned()
}

/// Client gRPC : toutes les operations de capture/restauration passent par la.
/// pi ci-dessus ne sert plus qu'a lire la configuration du serveur.
async fn grpc(ctx: &Context) -> Option<Arc<crate::shared::grpc_client::SentinelGrpcClient>> {
    ctx.data
        .read()
        .await
        .get::<crate::shared::grpc_client::GrpcClientKey>()
        .cloned()
}

/// Dispatch d'un event brut (`{"event":..,"data":..}`). Ignore silencieusement
/// tout event qui n'est pas destine a ce module.
async fn handle_event(ctx: &Context, payload_json: &str) {
    let envelope: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    let event = envelope.get("event").and_then(|v| v.as_str()).unwrap_or("");
    let data = match envelope.get("data") {
        Some(d) => d,
        None => return,
    };

    match event {
        EVENT_CAPTURE => on_capture_requested(ctx, data).await,
        EVENT_RESTORE => on_restore_requested(ctx, data).await,
        _ => {} // pas pour nous
    }
}

/// Parse un `guild_id` (chaine) en [`GuildId`]. `None` si invalide.
fn parse_guild(data: &serde_json::Value) -> Option<GuildId> {
    let s = data.get("guild_id").and_then(|v| v.as_str())?;
    s.parse::<u64>().ok().map(GuildId::new)
}

// ── capture_requested ──

async fn on_capture_requested(ctx: &Context, data: &serde_json::Value) {
    let Some(guild_id) = parse_guild(data) else {
        warn!("guild_backup(event): capture_requested sans guild_id valide");
        return;
    };
    let gid = guild_id.to_string();
    // La capture n'est pas destructive, mais elle consomme le quota de
    // snapshots : sans signature, un tiers capable d'ecrire sur le bus evincait
    // les sauvegardes reelles en en declenchant en boucle.
    if !event_signing::verifie(data, &event_signing::guild_backup_capture_message(&gid)) {
        warn!(guild = %gid, "guild_backup(event): signature invalide ou absente -> capture REJETEE");
        return;
    }
    let requested_by = data
        .get("requested_by")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let label = data
        .get("label")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            format!(
                "Sauvegarde du {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M")
            )
        });

    let (Some(api), Some(grpc)) = (api(ctx).await, grpc(ctx).await) else {
        warn!(guild = %gid, "guild_backup(event): API indisponible, capture ignoree");
        return;
    };

    let config = Config::load(&api, &gid).await;
    if !config.enabled() {
        info!(guild = %gid, "guild_backup(event): composant desactive, capture ignoree");
        return;
    }

    let snapshot = match capture::capture(ctx, guild_id, &label, &requested_by).await {
        Ok(s) => s,
        Err(e) => {
            warn!(guild = %gid, error = %e, "guild_backup(event): capture impossible");
            return;
        }
    };

    match api_client::store_snapshot(&grpc, &gid, &snapshot).await {
        Ok(id) => {
            info!(
                guild = %gid,
                snapshot_id = %id,
                roles = snapshot.roles.len(),
                channels = snapshot.channels.len(),
                "guild_backup(event): capture stockee (pilotage web)"
            );
            enforce_quota(&grpc, &gid, config.snapshot_quota()).await;
        }
        Err(e) => {
            warn!(guild = %gid, error = %e, "guild_backup(event): stockage capture impossible")
        }
    }
}

/// Elaguer les snapshots au-dela du quota configure (les plus anciens d'abord).
/// Best-effort : chaque echec est logge sans interrompre. NB: l'API applique
/// aussi son propre quota — celui-ci ne fait que resserrer si plus petit.
async fn enforce_quota(
    grpc: &Arc<crate::shared::grpc_client::SentinelGrpcClient>,
    guild_id: &str,
    quota: u64,
) {
    if quota == 0 {
        return;
    }
    let mut list = match api_client::list_snapshots(grpc, guild_id).await {
        Ok(l) => l,
        Err(e) => {
            warn!(guild = %guild_id, error = %e, "guild_backup(event): liste snapshots impossible (quota)");
            return;
        }
    };
    if list.len() as u64 <= quota {
        return;
    }
    // Tri anti-chronologique (created_at rfc3339 -> lexicographique) : on garde
    // les `quota` plus recents, on supprime le reste.
    list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    for stale in list.into_iter().skip(quota as usize) {
        match api_client::delete_snapshot(grpc, &stale.id).await {
            Ok(()) => {
                info!(guild = %guild_id, snapshot_id = %stale.id, "guild_backup(event): snapshot elague (quota)")
            }
            Err(e) => {
                warn!(guild = %guild_id, snapshot_id = %stale.id, error = %e, "guild_backup(event): elagage impossible")
            }
        }
    }
}

// ── restore_requested ──

async fn on_restore_requested(ctx: &Context, data: &serde_json::Value) {
    let Some(guild_id) = parse_guild(data) else {
        warn!("guild_backup(event): restore_requested sans guild_id valide");
        return;
    };
    let gid = guild_id.to_string();
    let Some(snapshot_id) = data.get("snapshot_id").and_then(|v| v.as_str()) else {
        warn!(guild = %gid, "guild_backup(event): restore_requested sans snapshot_id");
        return;
    };
    let snapshot_id = snapshot_id.to_string();
    let wipe_first = data.get("wipe").and_then(|v| v.as_bool()).unwrap_or(false);

    // Signature HMAC AVANT toute action : avec `wipe`, cet event supprime tous
    // les salons, roles et emojis du serveur. Le bus Redis est commun a toutes
    // les plateformes, donc en ecriture pour six processus plus la gateway :
    // seule la signature atteste que la demande vient bien de l'API.
    // `wipe` est dans le message signe -> impossible de rejouer une restauration
    // legitime en basculant le drapeau.
    let message = event_signing::guild_backup_restore_message(&gid, &snapshot_id, wipe_first);
    if !event_signing::verifie(data, &message) {
        warn!(
            guild = %gid,
            snapshot_id = %snapshot_id,
            wipe = wipe_first,
            "guild_backup(event): signature invalide ou absente -> restore REJETE"
        );
        return;
    }

    let (Some(api), Some(grpc)) = (api(ctx).await, grpc(ctx).await) else {
        warn!(guild = %gid, "guild_backup(event): API indisponible, restore ignore");
        return;
    };

    let config = Config::load(&api, &gid).await;
    if !config.enabled() {
        info!(guild = %gid, "guild_backup(event): composant desactive, restore ignore");
        return;
    }
    // NB : sur le chemin EVENT, l'autorisation est faite cote API (Bearer puis
    // gate superadmin) et attestee ici par la signature. `restore_role_ids`
    // n'est pas re-verifie (le reglage n'a jamais ete applique nulle part).

    let snapshot = match api_client::get_snapshot(&grpc, &snapshot_id).await {
        Ok(s) => s,
        Err(e) => {
            warn!(guild = %gid, snapshot_id = %snapshot_id, error = %e, "guild_backup(event): snapshot introuvable");
            return;
        }
    };

    // Repart propre : purge les re-attributions en attente d'un restore precedent.
    if let Err(e) = api_client::clear_pending_roles(&grpc, &gid).await {
        warn!(guild = %gid, error = %e, "guild_backup(event): purge pending-roles impossible");
    }

    let progress = ProgressSink::headless(gid.clone());

    let wipe_report = if wipe_first {
        Some(wipe::wipe(ctx, guild_id, &progress).await)
    } else {
        None
    };

    // Sans wipe -> mode merge (reutilise l'existant par nom, pas de doublon).
    let report = restore::restore(ctx, guild_id, &snapshot, !wipe_first, &progress).await;

    // Persiste les re-attributions pour les membres absents (re-rolises au retour).
    if !report.pending_grants.is_empty() {
        match api_client::save_pending_roles(&grpc, &gid, &report.pending_grants).await {
            Ok(n) => {
                info!(guild = %gid, saved = n, "guild_backup(event): pending-roles enregistres")
            }
            Err(e) => {
                warn!(guild = %gid, error = %e, "guild_backup(event): enregistrement pending-roles impossible")
            }
        }
    }

    info!(
        guild = %gid,
        snapshot_id = %snapshot_id,
        wiped = wipe_first,
        roles_created = report.roles_created,
        channels_created = report.channels_created,
        members_updated = report.members_updated,
        wipe_channels = wipe_report.map(|w| w.channels_deleted).unwrap_or(0),
        "guild_backup(event): restauration terminee (pilotage web)"
    );
}
