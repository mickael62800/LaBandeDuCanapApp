//! Module ai-dataset-bot : collecte autonome des messages texte pour
//! entrainer des modeles IA. Totalement independant des modules audit
//! et automod.
//!
//! Toggle par guild : `is_module_enabled(ctx, gid, "ai-dataset-bot")`.
//! Desactive par defaut. Quand actif, chaque message non-bot est pousse
//! via gRPC `AiDatasetService.CollectMessages` (client-streaming) qui
//! l'insere dans la table `ai_dataset_messages`.
//!
//! Architecture : `on_message` n'ouvre PAS un appel gRPC par message. Il
//! pousse le message dans un canal mpsc (`try_send`, non bloquant) dont le
//! Sender vit dans le TypeMap. Une task de fond (`spawn_collector`) relaie
//! ce canal vers une stream gRPC longue duree, reetablie automatiquement en
//! cas de rupture. Best-effort : la perte d'un message (buffer plein ou
//! reconnexion) est toleree.
//!
//! La page web "Dataset IA" lit cette table pour permettre l'etiquetage
//! manuel et l'export CSV.

use std::time::Duration;

use serenity::model::channel::Message;
use serenity::prelude::*;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;

use crate::shared::discord_helpers::is_module_enabled;
use crate::shared::grpc_client::GrpcClientKey;

use platform_proto::sentinel::ai_dataset::v1 as proto;

pub const MODULE_BOT_NAME: &str = "ai-dataset-bot";

/// Taille du canal stable bot -> collector. Au-dela, `try_send` jette le
/// message (back-pressure best-effort, on ne bloque jamais le hot path).
const CHANNEL_BUFFER: usize = 1024;
/// Backoff avant reconnexion de la stream apres une rupture.
const RECONNECT_BACKOFF: Duration = Duration::from_secs(5);

/// Cle TypeMap du Sender vers la stream de collecte.
struct AiDatasetSenderKey;
impl TypeMapKey for AiDatasetSenderKey {
    type Value = mpsc::Sender<proto::CollectMessageRequest>;
}

/// Demarre la task de collecte (a appeler une fois au `ready`).
///
/// Cree le canal stable, range son Sender dans le TypeMap, puis lance la
/// boucle qui maintient la stream gRPC `CollectMessages` ouverte.
pub async fn spawn_collector(ctx: Context) {
    let grpc = {
        let data = ctx.data.read().await;
        match data.get::<GrpcClientKey>() {
            Some(g) => g.clone(),
            None => return,
        }
    };

    let (tx, mut rx) = mpsc::channel::<proto::CollectMessageRequest>(CHANNEL_BUFFER);
    ctx.data.write().await.insert::<AiDatasetSenderKey>(tx);

    tokio::spawn(async move {
        loop {
            // Canal interne propre a CETTE connexion : alimente la stream gRPC.
            let (inner_tx, inner_rx) =
                mpsc::channel::<proto::CollectMessageRequest>(CHANNEL_BUFFER);
            let mut client = grpc.ai_dataset();
            let call = tokio::spawn(async move {
                client.collect_messages(ReceiverStream::new(inner_rx)).await
            });

            // Relaie le canal stable vers la stream courante jusqu'a rupture.
            loop {
                match rx.recv().await {
                    Some(msg) => {
                        if inner_tx.send(msg).await.is_err() {
                            break; // la stream s'est terminee (erreur reseau cote call)
                        }
                    }
                    None => {
                        // Sender stable droppe = arret du bot : on ferme proprement.
                        drop(inner_tx);
                        let _ = call.await;
                        return;
                    }
                }
            }

            match call.await {
                Ok(Ok(_)) => {}
                Ok(Err(status)) => {
                    warn!(code = ?status.code(), "Stream ai-dataset rompue, reconnexion");
                }
                Err(_) => {} // task annulee
            }
            tokio::time::sleep(RECONNECT_BACKOFF).await;
        }
    });
}

/// Pousse chaque message texte vers la stream de collecte si le module est
/// active sur la guild. Ignore les messages vides et les DMs.
pub async fn on_message(ctx: &Context, msg: &Message) {
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return, // Ignorer les DMs
    };

    // Filtre rapide avant de payer le cout de la requete config.
    let content = msg.content.trim();
    if content.is_empty() {
        return;
    }

    if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
        return;
    }

    // Resout le nom du salon (best-effort, ne bloque pas si echoue).
    let channel_name = msg
        .channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .map(|c| c.name.clone());

    let data = ctx.data.read().await;
    let tx = match data.get::<AiDatasetSenderKey>() {
        Some(tx) => tx.clone(),
        None => return, // collector pas encore demarre
    };
    drop(data);

    let req = proto::CollectMessageRequest {
        guild_id: guild_id.to_string(),
        channel_id: msg.channel_id.to_string(),
        channel_name,
        user_id: msg.author.id.to_string(),
        content: content.to_string(),
    };

    // Non bloquant : on jette le message si le buffer est plein (best-effort).
    let _ = tx.try_send(req);
}
