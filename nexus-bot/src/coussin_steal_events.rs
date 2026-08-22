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

#[derive(Debug, PartialEq)]
pub struct ParsedStealEvent<'a> {
    pub channel_id: u64,
    pub thief_id: &'a str,
    pub victim_id: &'a str,
    pub message_id: Option<u64>,
    pub success: bool,
    pub amount: i64,
    pub thief_total: i64,
    pub victim_total: i64,
    pub malus: i64,
}

pub fn parse_steal_event<'a>(envelope: &'a serde_json::Value) -> Option<ParsedStealEvent<'a>> {
    if envelope.get("event").and_then(|v| v.as_str())
        != Some(platform_core::nexus::ports::outbound::events::coussin_events::STEAL_RESOLVED)
    {
        return None;
    }
    let data = envelope.get("data")?;
    let channel_id = data.get("channel_id")?.as_str()?.parse::<u64>().ok()?;
    let thief_id = data.get("thief_id")?.as_str()?;
    let victim_id = data.get("victim_id")?.as_str()?;
    let message_id = data
        .get("message_id")
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<u64>().ok());
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

    Some(ParsedStealEvent {
        channel_id,
        thief_id,
        victim_id,
        message_id,
        success,
        amount,
        thief_total,
        victim_total,
        malus,
    })
}

pub fn process_steal_event_payload(payload: &str) -> Option<(u64, Option<u64>, String)> {
    let envelope = serde_json::from_str::<serde_json::Value>(payload).ok()?;
    let parsed = parse_steal_event(&envelope)?;
    let recit = format_recit(
        parsed.success,
        parsed.victim_id,
        parsed.thief_id,
        parsed.amount,
        parsed.thief_total,
        parsed.victim_total,
        parsed.malus,
    );
    Some((parsed.channel_id, parsed.message_id, recit))
}

async fn handle_event(ctx: &Context, payload: &str) {
    let Some((channel_id, message_id, recit)) = process_steal_event_payload(payload) else {
        return;
    };

    let channel = ChannelId::new(channel_id);

    // On edite le message d'origine quand on le connait : les boutons doivent
    // disparaitre avec la fenetre, sinon ils invitent a cliquer dans le vide.
    if let Some(msg_id) = message_id {
        let edited = channel
            .edit_message(
                &ctx.http,
                MessageId::new(msg_id),
                EditMessage::new().content(recit.clone()).components(vec![]),
            )
            .await;
        if edited.is_ok() {
            return;
        }
        warn!(
            %channel,
            message_id = msg_id, "fouille : message d'origine introuvable, publication a la suite"
        );
    }

    if let Err(error) = channel
        .send_message(&ctx.http, CreateMessage::new().content(recit))
        .await
    {
        warn!(%error, %channel, "fouille : denouement non publie");
    }
}

pub fn format_recit(
    success: bool,
    victim_id: &str,
    thief_id: &str,
    amount: i64,
    thief_total: i64,
    victim_total: i64,
    malus: i64,
) -> String {
    if success {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_recit_success() {
        let text = format_recit(true, "victim_1", "thief_1", 100, 15, 10, 2);
        assert!(text.contains("repart avec **100** coins"));
        assert!(text.contains("vigilance −2"));
    }

    #[test]
    fn test_format_recit_failure() {
        let text = format_recit(false, "victim_1", "thief_1", 50, 10, 18, 0);
        assert!(text.contains("reparti bredouille"));
        assert!(text.contains("**50** coins perdus"));
    }

    #[test]
    fn test_parse_steal_event() {
        let val = serde_json::json!({
            "event": "coussin_steal_resolved",
            "data": {
                "channel_id": "12345",
                "thief_id": "t1",
                "victim_id": "v1",
                "message_id": "67890",
                "success": true,
                "amount": 100,
                "thief_total": 20,
                "victim_total": 10,
                "absence_malus": 5
            }
        });
        let parsed = parse_steal_event(&val).unwrap();
        assert_eq!(parsed.channel_id, 12345);
        assert_eq!(parsed.thief_id, "t1");
        assert_eq!(parsed.victim_id, "v1");
        assert_eq!(parsed.message_id, Some(67890));
        assert!(parsed.success);
        assert_eq!(parsed.amount, 100);

        let invalid_event = serde_json::json!({
            "event": "other.event"
        });
        assert_eq!(parse_steal_event(&invalid_event), None);

        let missing_data = serde_json::json!({
            "event": "coussin_steal_resolved"
        });
        assert_eq!(parse_steal_event(&missing_data), None);

        let missing_channel = serde_json::json!({
            "event": "coussin_steal_resolved",
            "data": {
                "thief_id": "t1",
                "victim_id": "v1"
            }
        });
        assert_eq!(parse_steal_event(&missing_channel), None);
    }

    #[test]
    fn test_process_steal_event_payload() {
        // Invalid JSON
        assert_eq!(process_steal_event_payload("not json"), None);

        // Non-steal event
        assert_eq!(
            process_steal_event_payload(r#"{"event":"other.event"}"#),
            None
        );

        // Valid steal event
        let val_json = r#"{
            "event": "coussin_steal_resolved",
            "data": {
                "channel_id": "12345",
                "thief_id": "t1",
                "victim_id": "v1",
                "message_id": "67890",
                "success": true,
                "amount": 100,
                "thief_total": 20,
                "victim_total": 10,
                "absence_malus": 5
            }
        }"#;
        let res = process_steal_event_payload(val_json);
        assert!(res.is_some());
        let (channel_id, msg_id, recit) = res.unwrap();
        assert_eq!(channel_id, 12345);
        assert_eq!(msg_id, Some(67890));
        assert!(recit.contains("**100** coins"));
    }
}
