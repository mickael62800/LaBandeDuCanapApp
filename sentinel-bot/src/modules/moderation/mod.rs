//! Module moderation — 21 commandes slash + boutons + autocomplete + consumer Redis
//! (ex moderation-bot).

pub const MODULE_BOT_NAME: &str = "moderation-bot";

pub mod api_client;
pub mod appeal_behavior;
pub mod commands;
mod guild_reset;
mod pending_actions;
pub mod reason_templates;
mod redis_events;
pub mod risk_check;
mod risky_buttons;
pub mod role_mute;

use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use serenity::all::{
    AutocompleteChoice, CommandDataOptionValue, CommandInteraction, ComponentInteraction, Context,
    CreateAutocompleteResponse, CreateInteractionResponse,
};
use serenity::builder::CreateCommand;
use serenity::prelude::*;
use tracing::warn;

use crate::shared::api_client::BaseApiClient;
use crate::shared::discord_helpers::{
    is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
};
use crate::shared::heartbeat::ApiClientKey;

use api_client::{ApiClient, ModerationAction};

// ── TypeMapKeys ──

pub struct ModerationApiKey;
impl TypeMapKey for ModerationApiKey {
    type Value = Arc<ApiClient>;
}

pub struct PendingAction {
    pub action: ModerationAction,
    pub proposed_at: Instant,
}

pub struct PendingActionsKey;
impl TypeMapKey for PendingActionsKey {
    type Value = DashMap<String, PendingAction>;
}

pub const APPROVE_PREFIX: &str = "sentinel_mod_approve_";
pub const REJECT_PREFIX: &str = "sentinel_mod_reject_";

// ── Init TypeMapKeys ──

pub fn init_typemap(
    data: &mut serenity::prelude::TypeMap,
    api: &Arc<crate::shared::api_client::BaseApiClient>,
    grpc: &Arc<crate::shared::grpc_client::SentinelGrpcClient>,
) {
    data.insert::<ModerationApiKey>(Arc::new(ApiClient::new(Arc::clone(api), Arc::clone(grpc))));
    data.insert::<PendingActionsKey>(DashMap::new());
    data.insert::<risk_check::RiskyPendingKey>(DashMap::new());
}

// ── Slash commands ──

pub fn register_commands() -> Vec<CreateCommand> {
    commands::all()
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    let cmd_name = command.data.name.clone();
    let moderator = command.user.name.clone();
    let guild_id = command.guild_id.map(|g| g.to_string()).unwrap_or_default();

    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }

    match cmd_name.as_str() {
        "warn" => commands::warn::handle(ctx, command).await,
        "unwarn" => commands::unwarn::handle(ctx, command).await,
        "mute" => commands::mute::handle(ctx, command).await,
        "unmute" => commands::mute::handle_unmute(ctx, command).await,
        "ban" => commands::ban::handle(ctx, command).await,
        "ban-sursis" => commands::ban_sursis::handle(ctx, command).await,
        "unban" => commands::ban::handle_unban(ctx, command).await,
        "kick" => commands::kick::handle(ctx, command).await,
        "lock" => commands::channel_control::handle_lock(ctx, command).await,
        "unlock" => commands::channel_control::handle_unlock(ctx, command).await,
        "slowmode" => commands::channel_control::handle_slowmode(ctx, command).await,
        "history" => commands::history::handle(ctx, command).await,
        "call" => commands::call::handle(ctx, command).await,
        "signalement" => commands::card::handle(ctx, command).await,
        "context" => commands::context::handle(ctx, command).await,
        "appeal" => commands::appeal::handle(ctx, command).await,
        "compare" => commands::compare::handle(ctx, command).await,
        "evidence" => commands::evidence::handle(ctx, command).await,
        "review" => commands::review::handle(ctx, command).await,
        "template" => commands::template::handle(ctx, command).await,
        "transcript" => commands::transcript::handle(ctx, command).await,
        "export" => commands::export::handle(ctx, command).await,
        "massmute" => commands::mass::handle_massmute(ctx, command).await,
        "massban" => commands::mass::handle_massban(ctx, command).await,
        _ => {}
    }

    let data = ctx.data.read().await;
    if let Some(api) = data.get::<ApiClientKey>() {
        api.send_log(
            "info",
            &guild_id,
            &format!("Commande /{} executee par {}", cmd_name, moderator),
        );
    }
}

// ── Component interactions ──

pub fn handles_component(cid: &str) -> bool {
    cid.starts_with(commands::unwarn::UNWARN_PREFIX)
        || cid.starts_with(commands::call::CALL_CLOSE_PREFIX)
        || cid.starts_with(commands::appeal::APPEAL_PREFIX)
        || cid.starts_with(commands::appeal::APPEAL_VOTE_PREFIX)
        || cid.starts_with(commands::appeal::APPEAL_VALIDATE_PREFIX)
        || cid.starts_with(commands::appeal::APPEAL_BANCLOSE_PREFIX)
        || cid.starts_with(commands::appeal::APPEAL_BANCONFIRM_PREFIX)
        || cid == commands::appeal::APPEAL_CLOSE_ID
        || cid.starts_with(commands::ban_sursis::SURSIS_PARDON_PREFIX)
        || cid.starts_with(commands::ban_sursis::SURSIS_BAN_PREFIX)
        || cid.starts_with(APPROVE_PREFIX)
        || cid.starts_with(REJECT_PREFIX)
        || cid.starts_with(risk_check::CONFIRM_PREFIX)
        || cid.starts_with(risk_check::CANCEL_PREFIX)
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }
    let custom_id = &component.data.custom_id;

    if custom_id.starts_with(commands::unwarn::UNWARN_PREFIX) {
        commands::unwarn::handle_button(ctx, component).await;
    } else if custom_id.starts_with(commands::call::CALL_CLOSE_PREFIX) {
        commands::call::handle_close(ctx, component).await;
    } else if custom_id.starts_with(commands::appeal::APPEAL_VOTE_PREFIX) {
        commands::appeal::handle_vote_cancel(ctx, component).await;
    } else if custom_id.starts_with(commands::appeal::APPEAL_VALIDATE_PREFIX) {
        commands::appeal::handle_validate_cancel(ctx, component).await;
    } else if custom_id.starts_with(commands::appeal::APPEAL_BANCONFIRM_PREFIX) {
        commands::appeal::handle_ban_confirm(ctx, component).await;
    } else if custom_id.starts_with(commands::appeal::APPEAL_BANCLOSE_PREFIX) {
        commands::appeal::handle_ban_close(ctx, component).await;
    } else if custom_id == commands::appeal::APPEAL_CLOSE_ID {
        commands::appeal::handle_appeal_close(ctx, component).await;
    } else if custom_id.starts_with(commands::ban_sursis::SURSIS_PARDON_PREFIX) {
        commands::ban_sursis::handle_pardon(ctx, component).await;
    } else if custom_id.starts_with(commands::ban_sursis::SURSIS_BAN_PREFIX) {
        commands::ban_sursis::handle_ban_now(ctx, component).await;
    } else if custom_id.starts_with(commands::appeal::APPEAL_PREFIX) {
        commands::appeal::handle_appeal_button(ctx, component).await;
    } else if custom_id.starts_with(APPROVE_PREFIX) {
        pending_actions::handle_approve(ctx, component).await;
    } else if custom_id.starts_with(REJECT_PREFIX) {
        pending_actions::handle_reject(ctx, component).await;
    } else if custom_id.starts_with(risk_check::CONFIRM_PREFIX) {
        risky_buttons::handle_risky_confirm(ctx, component).await;
    } else if custom_id.starts_with(risk_check::CANCEL_PREFIX) {
        risky_buttons::handle_risky_cancel(ctx, component).await;
    }
}

// ── Autocomplete (reason templates) ──

pub fn handles_autocomplete(cmd_name: &str) -> bool {
    matches!(cmd_name, "warn" | "mute" | "ban")
}

pub async fn handle_autocomplete(ctx: &Context, autocomplete: &CommandInteraction) {
    let guild_id = autocomplete
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();

    let current_input = autocomplete
        .data
        .options
        .iter()
        .find(|o| o.name == "reason")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("");

    let templates_raw = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let gc = match base.get_guild_config_for(&guild_id, MODULE_BOT_NAME).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch guild config for reason templates");
                    std::collections::HashMap::new()
                }
            };
            BaseApiClient::config_or(&gc, "reason_templates", "")
        } else {
            String::new()
        }
    };

    let templates = reason_templates::parse_templates(&templates_raw);
    let filtered = reason_templates::filter_templates(&templates, current_input);

    let choices: Vec<AutocompleteChoice> = filtered
        .iter()
        .map(|t| AutocompleteChoice::new(&t.label, serde_json::Value::String(t.reason.clone())))
        .collect();

    let response = CreateAutocompleteResponse::new().set_choices(choices);

    if let Err(e) = autocomplete
        .create_response(&ctx.http, CreateInteractionResponse::Autocomplete(response))
        .await
    {
        warn!(error = %e, "Failed to send autocomplete response");
    }
}

/// Cree un salon d'appel PRIVE sous la categorie `appeal_category_id` : visible
/// seulement par l'appelant et le role moderateur. Retourne l'id du salon cree,
/// ou `None` si la categorie n'est pas configuree (l'appelant retombe alors sur
/// la notification `appeal_channel_id`).
pub async fn create_appeal_channel(
    ctx: &Context,
    guild_id: &str,
    appellant_id: u64,
    appellant_name: &str,
    intro: serenity::builder::CreateEmbed,
    buttons: Vec<serenity::builder::CreateButton>,
) -> Option<serenity::model::id::ChannelId> {
    use serenity::all::{
        ChannelType, CreateChannel, PermissionOverwrite, PermissionOverwriteType, Permissions,
    };
    use serenity::model::id::{ChannelId, GuildId, RoleId, UserId};

    let gid: u64 = guild_id.parse().ok()?;
    let guild = GuildId::new(gid);

    let cfg = {
        let data = ctx.data.read().await;
        let api = data.get::<ApiClientKey>()?;
        api.get_guild_config_for(guild_id, MODULE_BOT_NAME)
            .await
            .unwrap_or_default()
    };

    // Categorie requise : sans elle, on ne cree pas de salon (fallback appelant).
    let category_id = cfg
        .get("appeal_category_id")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&n| n > 0)?;

    // Permissions : @everyone ne voit rien, l'appelant et le role modo voient.
    let mut overwrites = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: PermissionOverwriteType::Role(RoleId::new(gid)), // @everyone
        },
        PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL
                | Permissions::SEND_MESSAGES
                | Permissions::READ_MESSAGE_HISTORY,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(UserId::new(appellant_id)),
        },
    ];
    for key in ["moderator_role_id", "mod_role_id"] {
        if let Some(rid) = cfg
            .get(key)
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&n| n > 0)
        {
            overwrites.push(PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::SEND_MESSAGES
                    | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Role(RoleId::new(rid)),
            });
            break;
        }
    }

    // Nom de salon : appel-<pseudo assaini>.
    let slug: String = appellant_name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(90)
        .collect();
    let channel_name = format!(
        "appel-{}",
        if slug.is_empty() {
            "membre".into()
        } else {
            slug
        }
    );

    let create = CreateChannel::new(&channel_name)
        .kind(ChannelType::Text)
        .category(ChannelId::new(category_id))
        .topic(format!("Appel de sanction — {appellant_name}"))
        .permissions(overwrites);

    let channel = match guild.create_channel(&ctx.http, create).await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, guild_id, "Echec creation salon d'appel");
            return None;
        }
    };

    // Message d'accroche : ping l'appelant + embed d'intro + boutons modo fournis.
    use serenity::all::CreateActionRow;
    let mut msg = serenity::builder::CreateMessage::new()
        .content(format!("<@{appellant_id}>"))
        .embed(intro);
    if !buttons.is_empty() {
        msg = msg.components(vec![CreateActionRow::Buttons(buttons)]);
    }
    let _ = channel.send_message(&ctx.http, msg).await;
    appeal_behavior::initialize(ctx, channel.id, UserId::new(appellant_id)).await;

    Some(channel.id)
}

// ── Background tasks ──

/// Spawn le consumer Redis des events moderation (appele depuis ready).
pub fn spawn_background(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "moderation-bot".to_string(),
            consumer,
            move |payload| {
                let ctx = ctx.clone();
                async move {
                    redis_events::handle_redis_moderation_event(&ctx, &payload).await;
                    guild_reset::handle_guild_reset_event(&ctx, &payload).await;
                }
            },
        )
        .await;
    });
}
