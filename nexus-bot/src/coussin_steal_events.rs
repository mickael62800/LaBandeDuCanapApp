//! Recit des fouilles tranchees faute de reaction.
//!
//! Quand la victime laisse passer la fenetre, c'est le job de l'API qui
//! tranche — le bot n'etait pas dans la boucle. Sans ce consumer, la fenetre
//! se refermerait dans le silence : la victime verrait son solde baisser sans
//! explication, et le voleur ne saurait jamais s'il a reussi.
//!
//! Le message est publie dans le salon d'origine, retrouve par l'evenement et
//! non par une variable en memoire : le bot a pu redemarrer pendant la fenetre.

use std::sync::Arc;

use serenity::all::{ChannelId, Context, CreateMessage, EditMessage, MessageId};
use tracing::warn;

use crate::api_client::ApiClient;

pub fn spawn(ctx: Context, api: Arc<ApiClient>) {
    tokio::spawn(async move {
        crate::event_bus::listen_stream_group(
            "nexus-bot-coussin-steal".to_string(),
            crate::event_bus::default_consumer_name(),
            move |payload_json| {
                let ctx = ctx.clone();
                let _api = api.clone();
                async move { handle_event(&ctx, &payload_json).await }
            },
        )
        .await;
    });
}

async fn handle_event(ctx: &Context, payload: &str) {
    let Ok(envelope) = serde_json::from_str::<serde_json::Value>(payload) else {
        return;
    };
    if envelope.get("event").and_then(|v| v.as_str())
        != Some(platform_core::nexus::ports::outbound::events::coussin_events::STEAL_RESOLVED)
    {
        return;
    }
    let Some(data) = envelope.get("data") else {
        return;
    };

    let (Some(channel_id), Some(thief_id), Some(victim_id)) = (
        data.get("channel_id")
            .and_then(|v| v.as_str())
            .and_then(|v| v.parse::<u64>().ok()),
        data.get("thief_id").and_then(|v| v.as_str()),
        data.get("victim_id").and_then(|v| v.as_str()),
    ) else {
        return;
    };

    let success = data
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let amount = data.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
    let thief_total = data
        .get("thief_total")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let victim_total = data
        .get("victim_total")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let malus = data
        .get("absence_malus")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    // Dire pourquoi, pas seulement quoi : la victime doit comprendre que son
    // silence a pese, sinon la fenetre de defense n'apprend rien a personne.
    let recit = if success {
        format!(
            "😴 <@{victim_id}> n'a pas bougé… <@{thief_id}> repart avec **{amount}** coins.\n\
             🎲 Voleur : **{thief_total}** — Défense : **{victim_total}** (vigilance −{malus})"
        )
    } else {
        format!(
            "🍀 <@{victim_id}> n'a pas réagi, mais <@{thief_id}> est reparti bredouille : \
             **{amount}** coins perdus.\n\
             🎲 Voleur : **{thief_total}** — Défense : **{victim_total}** (vigilance −{malus})"
        )
    };

    let channel = ChannelId::new(channel_id);

    // On edite le message d'origine quand on le connait : les boutons doivent
    // disparaitre avec la fenetre, sinon ils invitent a cliquer dans le vide.
    if let Some(message_id) = data
        .get("message_id")
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<u64>().ok())
    {
        let edited = channel
            .edit_message(
                &ctx.http,
                MessageId::new(message_id),
                EditMessage::new().content(recit.clone()).components(vec![]),
            )
            .await;
        if edited.is_ok() {
            return;
        }
        warn!(
            channel_id,
            message_id, "fouille : message d'origine introuvable, publication a la suite"
        );
    }

    if let Err(error) = channel
        .send_message(&ctx.http, CreateMessage::new().content(recit))
        .await
    {
        warn!(%error, channel_id, "fouille : denouement non publie");
    }
}
