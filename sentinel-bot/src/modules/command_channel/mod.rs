//! Salons "commandes uniquement" : supprime en silence tout message texte
//! classique dans les salons configures. Les commandes slash sont des
//! interactions (pas des messages) -> non affectees. L'owner et les bots ne
//! sont jamais supprimes.

pub const MODULE_BOT_NAME: &str = "command-channel-bot";

use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::debug;

use crate::shared::api_client::BaseApiClient;
use crate::shared::discord_helpers::{guild_config_or_default, is_module_enabled};

pub async fn on_message(ctx: &Context, msg: &Message) -> bool {
    // Messages de bots : jamais supprimes (reponses de commandes, panneaux).
    if msg.author.bot {
        return false;
    }
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return false, // pas en MP
    };
    let gid = guild_id.to_string();

    if !is_module_enabled(ctx, &gid, MODULE_BOT_NAME).await {
        return false;
    }

    let config = guild_config_or_default(ctx, &gid, MODULE_BOT_NAME).await;
    let channels = BaseApiClient::config_or(&config, "command_channels", "");
    if channels.is_empty() {
        return false;
    }
    let target = msg.channel_id.get().to_string();
    let is_command_channel = channels.split(',').map(|s| s.trim()).any(|c| c == target);
    if !is_command_channel {
        return false;
    }

    // L'owner du serveur peut ecrire librement.
    let is_owner = ctx
        .cache
        .guild(guild_id)
        .map(|g| g.owner_id == msg.author.id)
        .unwrap_or(false);
    if is_owner {
        return false;
    }

    // Efface silencieusement.
    if let Err(e) = msg.delete(&ctx.http).await {
        debug!(error = %e, "Echec suppression message hors-commande");
    }

    true
}
