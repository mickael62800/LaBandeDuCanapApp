//! Carte de suivi du comportement d'un membre pendant son appel.

use std::sync::LazyLock;

use dashmap::DashMap;
use serenity::all::{ChannelId, Context, CreateEmbed, EditMessage, MessageId, UserId};
use serenity::builder::CreateMessage;

#[derive(Clone)]
struct CardState {
    message_id: MessageId,
    count: u32,
    last_reason: String,
}
static CARDS: LazyLock<DashMap<(u64, u64), CardState>> = LazyLock::new(DashMap::new);

fn embed(count: u32, reason: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("⚠️ Comportement pendant l'appel")
        .description("Les messages problématiques sont supprimés. Cette carte informe le staff sans prolonger la sanction.")
        .field("Incidents depuis l'ouverture", count.to_string(), true)
        .field("Dernière détection", reason, false)
        .timestamp(serenity::model::Timestamp::now())
}

pub async fn initialize(ctx: &Context, channel: ChannelId, user: UserId) {
    if let Ok(message) = channel
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embed(0, "Aucun incident.")),
        )
        .await
    {
        CARDS.insert(
            (channel.get(), user.get()),
            CardState {
                message_id: message.id,
                count: 0,
                last_reason: "Aucun incident.".into(),
            },
        );
    }
}

pub async fn record(ctx: &Context, channel: ChannelId, user: UserId, reason: &str) {
    let key = (channel.get(), user.get());
    let Some(mut state) = CARDS.get_mut(&key) else {
        return;
    };
    state.count = state.count.saturating_add(1);
    state.last_reason = reason.chars().take(180).collect();
    let _ = channel
        .edit_message(
            &ctx.http,
            state.message_id,
            EditMessage::new().embed(embed(state.count, &state.last_reason)),
        )
        .await;
}
