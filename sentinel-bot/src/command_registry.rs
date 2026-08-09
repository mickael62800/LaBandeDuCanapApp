//! Registre central : mapping bot_name -> commandes slash + fonction
//! qui calcule les commandes a publier pour une guild en filtrant les
//! modules desactives.
//!
//! Utilise au boot et a chaque event "bot_enabled_changed" sur Redis stream
//! pour re-register les commandes via guild.set_application_commands(...).
//! Les commandes des modules desactives sont LITTERALEMENT retirees de
//! Discord, plus juste filtrees a l'execution.

use serenity::all::CreateCommand;
use serenity::model::id::GuildId;
use serenity::prelude::Context;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

use crate::modules;

/// Mapping bot_name (utilise dans bot_guild_config / page Composants)
/// vers le builder des commandes slash de ce module.
///
/// Bots qui n'ont pas de slash commands ne sont pas listes (ex audit-bot
/// ecoute juste les events Discord, automod-bot scan les messages, etc.).
pub(crate) fn module_commands(bot_name: &str) -> Vec<CreateCommand> {
    match bot_name {
        "cleanup" => modules::cleanup::register_commands(),
        "bump-bot" => modules::bump::register_commands(),
        "community-bot" => modules::community::register_commands(),
        "audit-bot" => modules::audit::register_commands(),
        "progression-bot" => modules::progression::register_commands(),
        "security-bot" => modules::security::register_commands(),
        "automod-bot" => modules::automod::register_commands(),
        "moderation-bot" => modules::moderation::register_commands(),
        "voice-bot" => modules::voice::register_commands(),
        "ticket-bot" => modules::tickets::register_commands(),
        "idea-bot" => modules::ideas::register_commands(),
        // confessions a deux commandes /confess et /confess-admin :
        // register_commands() existe sur le module mais avec attribut allow.
        // On l'utilise ici officiellement.
        "confessions" => modules::confessions::register_commands(),
        "guild-backup-bot" => modules::guild_backup::register_commands(),
        "nasa-apod-bot" => modules::nasa_apod::register_commands(),
        _ => Vec::new(),
    }
}

/// Liste de tous les bot_names qui ont des commandes slash. Utilise
/// au boot pour iterer.
pub const BOT_NAMES_WITH_COMMANDS: &[&str] = &[
    "cleanup",
    "bump-bot",
    "community-bot",
    "audit-bot",
    "progression-bot",
    "security-bot",
    "automod-bot",
    "moderation-bot",
    "voice-bot",
    "ticket-bot",
    "idea-bot",
    "confessions",
    "guild-backup-bot",
    "nasa-apod-bot",
];

/// Calcule la liste des commandes a enregistrer pour cette guild en
/// fonction de l'etat enabled de chaque module dans bot_guild_config.
/// Si la cle "enabled" n'est pas definie, le module est considere
/// active (defaut).
async fn compute_guild_commands(api: &BaseApiClient, guild_id: &str) -> Vec<CreateCommand> {
    // Socle : commandes d'INSTALLATION, publiees quel que soit l'etat des
    // modules.
    //
    // `/logs-init` cree les salons de logs et renseigne les reglages de cinq
    // modules a la fois. La rattacher a l'un d'eux la ferait disparaitre quand
    // il est desactive — or tout est desactive sur un serveur neuf, c'est-a-dire
    // exactement le moment ou on en a besoin.
    let mut commands = vec![crate::modules::logs_setup::register()];

    for bot_name in BOT_NAMES_WITH_COMMANDS {
        // is_bot_enabled retourne true par defaut (pas de cle = active).
        let enabled = crate::shared::discord_helpers::is_bot_enabled(api, guild_id, bot_name).await;
        if enabled {
            commands.extend(module_commands(bot_name));
        }
    }
    commands
}

/// Re-enregistre les commandes slash pour une guild precise. Discord
/// remplace TOUTES les commandes de la guild en une seule operation
/// atomique. Donc desactiver un module fait disparaitre instantanement
/// ses commandes du serveur Discord (pas besoin de redemarrer le bot).
pub async fn refresh_guild_commands(ctx: &Context, guild_id: GuildId) {
    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => {
                warn!("ApiClientKey absent, impossible de refresh les commandes guild");
                return;
            }
        }
    };
    let commands = compute_guild_commands(&api, &guild_id.to_string()).await;
    let count = commands.len();
    match guild_id.set_commands(&ctx.http, commands).await {
        Ok(_) => {
            info!(guild_id = %guild_id, commands = count, "Slash commands refreshed for guild")
        }
        Err(e) => warn!(error = %e, guild_id = %guild_id, "Echec refresh commandes guild"),
    }
}

/// Listener Redis stream : ecoute les events "bot_enabled_changed"
/// envoyes par l'API quand un admin toggle on/off un module via la
/// page Composants. Re-register les commandes de la guild concernee.
pub fn spawn_consumer(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "sentinel-bot-command-registry".to_string(),
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
    if envelope.get("event").and_then(|v| v.as_str()) != Some("bot_enabled_changed") {
        return;
    }
    let data = match envelope.get("data") {
        Some(d) => d,
        None => return,
    };
    let guild_id_str = data
        .get("guild_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let bot_name = data
        .get("bot_name")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let enabled = data
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let Ok(gid_u64) = guild_id_str.parse::<u64>() else {
        return;
    };
    let guild_id = GuildId::new(gid_u64);
    info!(guild_id = %guild_id, bot_name, enabled, "Bot enabled changed -> refreshing commands");
    refresh_guild_commands(ctx, guild_id).await;
}
