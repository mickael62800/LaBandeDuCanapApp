//! Consumer stream : poste / edite des embeds (builder style Carl-bot).
//!
//! Flow :
//! 1. L'API publie XADD `sentinel:events` event="embed_publish" quand un
//!    superadmin clique « Poster » ou « Mettre a jour » dans le builder web.
//! 2. Le bot (ce module) consume, construit la carte Discord (author, titre,
//!    description, couleur, image, thumbnail, footer, timestamp, champs), puis
//!    POSTE un nouveau message OU EDITE le message existant selon `message_id`.
//! 3. Apres un POST, rapporte (channel_id, message_id) a l'API via
//!    POST /api/embeds/by-id/{id}/posted pour permettre l'edition ulterieure.

use serde::{Deserialize, Serialize};
use serenity::all::{Color, EditMessage};
use serenity::builder::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage};
use serenity::model::id::{ChannelId, MessageId};
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::grpc_client::{grpc_err_to_string, GrpcClientKey};
use platform_proto::sentinel::embeds::v1 as proto_embeds;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct EmbedField {
    name: String,
    value: String,
    #[serde(default)]
    inline: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RenderedEmbedPost {
    embed_id: String,
    guild_id: String,
    channel_id: String,
    #[serde(default)]
    message_id: Option<String>,
    content: String,
    author_name: String,
    author_icon_url: String,
    author_url: String,
    title: String,
    title_url: String,
    description: String,
    color: Option<i32>,
    image_url: String,
    thumbnail_url: String,
    footer_text: String,
    footer_icon_url: String,
    show_timestamp: bool,
    #[serde(default)]
    fields: Vec<EmbedField>,
}

/// Spawn le consumer durable. Appele une fois au `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "sentinel-bot-embeds".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move { handle_event(&ctx, &payload_json).await }
            },
        )
        .await;
    });
}

async fn handle_event(ctx: &Context, payload_json: &str) {
    let envelope: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    if envelope.get("event").and_then(|v| v.as_str()) != Some("embed_publish") {
        return;
    }
    let data = match envelope.get("data") {
        Some(d) => d.clone(),
        None => return,
    };
    let payload: RenderedEmbedPost = match serde_json::from_value(data) {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "embed_publish: payload invalide");
            return;
        }
    };

    let channel_id = match payload.channel_id.parse::<u64>() {
        Ok(id) => ChannelId::new(id),
        Err(_) => {
            warn!(channel = %payload.channel_id, "embed_publish: channel_id invalide");
            return;
        }
    };

    let embed = build_embed(&payload);

    // EDITION si message_id fourni, sinon POST d'un nouveau message.
    if let Some(msg_id_str) = &payload.message_id {
        let Ok(mid) = msg_id_str.parse::<u64>() else {
            warn!(message = %msg_id_str, "embed_publish: message_id invalide");
            return;
        };
        let mut edit = EditMessage::new().embeds(vec![embed]);
        if !payload.content.is_empty() {
            edit = edit.content(&payload.content);
        }
        match channel_id
            .edit_message(&ctx.http, MessageId::new(mid), edit)
            .await
        {
            Ok(_) => info!(embed_id = %payload.embed_id, "Embed edite"),
            Err(e) => warn!(embed_id = %payload.embed_id, error = %e, "Echec edition embed"),
        }
        return;
    }

    let mut msg = CreateMessage::new().embed(embed);
    if !payload.content.is_empty() {
        msg = msg.content(&payload.content);
    }
    match channel_id.send_message(&ctx.http, msg).await {
        Ok(message) => {
            info!(embed_id = %payload.embed_id, "Embed poste");
            report_posted(
                ctx,
                &payload.embed_id,
                &payload.channel_id,
                &message.id.to_string(),
            )
            .await;
        }
        Err(e) => warn!(embed_id = %payload.embed_id, error = %e, "Echec post embed"),
    }
}

fn build_embed(p: &RenderedEmbedPost) -> CreateEmbed {
    let mut e = CreateEmbed::new();
    if !p.title.is_empty() {
        e = e.title(&p.title);
        if !p.title_url.is_empty() {
            e = e.url(&p.title_url);
        }
    }
    if !p.description.is_empty() {
        e = e.description(&p.description);
    }
    if let Some(c) = p.color {
        e = e.color(Color::new(c as u32));
    }
    if !p.author_name.is_empty() {
        let mut author = CreateEmbedAuthor::new(&p.author_name);
        if !p.author_icon_url.is_empty() {
            author = author.icon_url(&p.author_icon_url);
        }
        if !p.author_url.is_empty() {
            author = author.url(&p.author_url);
        }
        e = e.author(author);
    }
    if !p.footer_text.is_empty() {
        let mut footer = CreateEmbedFooter::new(&p.footer_text);
        if !p.footer_icon_url.is_empty() {
            footer = footer.icon_url(&p.footer_icon_url);
        }
        e = e.footer(footer);
    }
    if !p.image_url.is_empty() {
        e = e.image(&p.image_url);
    }
    if !p.thumbnail_url.is_empty() {
        e = e.thumbnail(&p.thumbnail_url);
    }
    if p.show_timestamp {
        e = e.timestamp(serenity::model::Timestamp::now());
    }
    for f in &p.fields {
        if !f.name.trim().is_empty() {
            e = e.field(&f.name, &f.value, f.inline);
        }
    }
    e
}

async fn report_posted(ctx: &Context, embed_id: &str, channel_id: &str, message_id: &str) {
    let grpc = {
        let data = ctx.data.read().await;
        data.get::<GrpcClientKey>().cloned()
    };
    let Some(grpc) = grpc else { return };
    let req = proto_embeds::RecordPostedRequest {
        embed_id: embed_id.to_string(),
        channel_id: channel_id.to_string(),
        message_id: message_id.to_string(),
    };
    if let Err(e) = crate::grpc_call!(@unit &grpc, embeds, record_posted, req) {
        warn!(embed_id, error = %e, "Echec report embed posted");
    }
}
