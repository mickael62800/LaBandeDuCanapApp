use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::builder::CreateEmbedFooter;
use tracing::{info, warn};

use crate::shared::embeds::danger_embed;

use super::api_client::ModerationAction;
use super::ModerationApiKey;

pub fn register_massmute() -> CreateCommand {
    CreateCommand::new("massmute")
        .description("Mute plusieurs utilisateurs en une seule commande")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "users",
                "IDs des utilisateurs (separes par des espaces ou virgules)",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison du mute")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "duration",
                "Duree en minutes (defaut: 10)",
            )
            .min_int_value(1)
            .max_int_value(40320),
        )
}

pub fn register_massban() -> CreateCommand {
    CreateCommand::new("massban")
        .description("Bannir plusieurs utilisateurs en une seule commande")
        .default_member_permissions(serenity::all::Permissions::BAN_MEMBERS)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "users",
                "IDs des utilisateurs (separes par des espaces ou virgules)",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison du ban")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "purge",
                "Supprimer les messages recents des bannis (defaut 1 jour)",
            )
            .add_int_choice("Aucun message", 0)
            .add_int_choice("1 jour", 1)
            .add_int_choice("3 jours", 3)
            .add_int_choice("7 jours", 7),
        )
}

pub async fn handle_massmute(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::MODERATE_MEMBERS) {
        crate::shared::discord_helpers::reply_ephemeral(
            ctx,
            command,
            "âŒ Permission MODERATE_MEMBERS requise pour /massmute.",
        )
        .await;
        warn!(user = %command.user.name, "Tentative /massmute sans permission");
        return;
    }

    let users_str = command
        .data
        .options
        .iter()
        .find(|o| o.name == "users")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("");

    let reason = command
        .data
        .options
        .iter()
        .find(|o| o.name == "reason")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("Mass mute");

    let duration_min = command
        .data
        .options
        .iter()
        .find(|o| o.name == "duration")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(i) => Some(*i as u64),
            _ => None,
        })
        .unwrap_or(10);

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            crate::shared::discord_helpers::reply_ephemeral(
                ctx,
                command,
                "Commande serveur uniquement.",
            )
            .await;
            return;
        }
    };

    let user_ids = parse_user_ids(users_str);
    if user_ids.is_empty() {
        crate::shared::discord_helpers::reply_ephemeral(
            ctx,
            command,
            "Aucun ID utilisateur valide detecte.",
        )
        .await;
        return;
    }
    if user_ids.len() > 200 {
        crate::shared::discord_helpers::reply_ephemeral(
            ctx,
            command,
            "Maximum 200 utilisateurs par commande.",
        )
        .await;
        return;
    }

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!(
                        "Mute en cours de {} utilisateurs...",
                        user_ids.len()
                    ))
                    .ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Failed to send massmute initial response");
    }

    let duration_secs = duration_min * 60;
    let mut success = 0u32;
    let mut failures = 0u32;
    let mut immune = 0u32;

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => {
            tracing::error!("ModerationApiKey manquant");
            return;
        }
    };

    for uid in &user_ids {
        let user_id = serenity::model::id::UserId::new(*uid);
        if super::find_immune_role(ctx, guild_id, user_id)
            .await
            .is_some()
        {
            immune += 1;
            continue;
        }
        // Hierarchie : ne jamais sanctionner en masse soi / le bot / l'owner /
        // un membre de rang >= au moderateur (comptes comme "proteges").
        if super::check_hierarchy(ctx, command, guild_id, user_id).is_err() {
            immune += 1;
            continue;
        }
        match guild_id.member(&ctx.http, user_id).await {
            Ok(mut member) => {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
                    + duration_secs as i64;
                let datetime = time::OffsetDateTime::from_unix_timestamp(ts)
                    .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
                let timeout = serenity::model::Timestamp::from(datetime);

                if member
                    .disable_communication_until_datetime(&ctx.http, timeout)
                    .await
                    .is_ok()
                {
                    success += 1;
                    if let Err(e) = api
                        .log_action(&ModerationAction {
                            guild_id: guild_id.to_string(),
                            channel_id: command.channel_id.to_string(),
                            moderator_id: command.user.id.to_string(),
                            moderator_name: command.user.name.clone(),
                            target_id: uid.to_string(),
                            target_name: member.user.name.clone(),
                            action_type: "mute_temp".to_string(),
                            reason: reason.to_string(),
                            gravity: None,
                            duration: Some(duration_secs),
                        })
                        .await
                    {
                        warn!(error = %e, uid = %uid, "Failed to log massmute action");
                    }
                } else {
                    failures += 1;
                }
            }
            Err(_) => failures += 1,
        }
    }

    let embed = danger_embed(format!("Mass Mute — {} utilisateurs", user_ids.len()))
        .field("Moderateur", format!("<@{}>", command.user.id), true)
        .field("Reussi", success.to_string(), true)
        .field("Echoue", failures.to_string(), true)
        .field("Immunises", immune.to_string(), true)
        .field("Duree", format!("{}min", duration_min), true)
        .field("Raison", reason, false)
        .timestamp(serenity::model::Timestamp::now())
        .footer(CreateEmbedFooter::new("Moderation | Sentinel"));

    let followup = serenity::builder::CreateInteractionResponseFollowup::new()
        .content(format!(
            "âœ… Mass mute termine : {success}/{} utilisateurs.",
            user_ids.len()
        ))
        .ephemeral(true);
    if let Err(e) = command.create_followup(&ctx.http, followup).await {
        warn!(error = %e, "Failed to send mass mute followup");
    }

    super::log_to_channel(ctx, &guild_id.to_string(), embed).await;

    // BUG #4 : une seule card recapitulative (pas une par membre).
    crate::shared::discord_helpers::post_sanction_summary_card(
        ctx,
        &guild_id.to_string(),
        crate::shared::discord_helpers::SanctionKind::Mute,
        success,
        &command.user.name,
        reason,
    )
    .await;

    info!(
        moderator = %command.user.name,
        success, failures,
        total = user_ids.len(),
        "Mass mute execute"
    );
}

pub async fn handle_massban(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::BAN_MEMBERS) {
        crate::shared::discord_helpers::reply_ephemeral(
            ctx,
            command,
            "âŒ Permission BAN_MEMBERS requise pour /massban.",
        )
        .await;
        warn!(user = %command.user.name, "Tentative /massban sans permission");
        return;
    }

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, cmd = "massban", "Echec defer interaction Discord");
        return;
    }

    let users_str = command
        .data
        .options
        .iter()
        .find(|o| o.name == "users")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("");

    let reason = command
        .data
        .options
        .iter()
        .find(|o| o.name == "reason")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("Mass ban");

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            let _ = command
                .edit_response(
                    &ctx.http,
                    serenity::builder::EditInteractionResponse::new()
                        .content("Commande serveur uniquement."),
                )
                .await;
            return;
        }
    };

    let user_ids = parse_user_ids(users_str);
    if user_ids.is_empty() {
        let _ = command
            .edit_response(
                &ctx.http,
                serenity::builder::EditInteractionResponse::new()
                    .content("Aucun ID utilisateur valide detecte."),
            )
            .await;
        return;
    }
    if user_ids.len() > 200 {
        let _ = command
            .edit_response(
                &ctx.http,
                serenity::builder::EditInteractionResponse::new()
                    .content("Maximum 200 utilisateurs par commande."),
            )
            .await;
        return;
    }

    // Purge des messages (0/1/3/7 j) appliquee a tout le lot ; defaut 1 jour.
    let delete_days: u8 = command
        .data
        .options
        .iter()
        .find(|o| o.name == "purge")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(n) => Some((*n).clamp(0, 7) as u8),
            _ => None,
        })
        .unwrap_or(7);

    let _ = command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().content(format!(
                "Ban en cours de {} utilisateurs...",
                user_ids.len()
            )),
        )
        .await;

    let mut success = 0u32;
    let mut failures = 0u32;
    let mut immune = 0u32;

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => {
            tracing::error!("ModerationApiKey manquant");
            return;
        }
    };

    for uid in &user_ids {
        let user_id = serenity::model::id::UserId::new(*uid);
        if super::find_immune_role(ctx, guild_id, user_id)
            .await
            .is_some()
        {
            immune += 1;
            continue;
        }
        // Hierarchie : ne jamais sanctionner en masse soi / le bot / l'owner /
        // un membre de rang >= au moderateur (comptes comme "proteges").
        if super::check_hierarchy(ctx, command, guild_id, user_id).is_err() {
            immune += 1;
            continue;
        }
        let target_name = user_id
            .to_user(&ctx.http)
            .await
            .map(|u| u.name.clone())
            .unwrap_or_else(|_| uid.to_string());

        if guild_id
            .ban_with_reason(&ctx.http, user_id, delete_days, reason)
            .await
            .is_ok()
        {
            success += 1;
            if let Err(e) = api
                .log_action(&ModerationAction {
                    guild_id: guild_id.to_string(),
                    channel_id: command.channel_id.to_string(),
                    moderator_id: command.user.id.to_string(),
                    moderator_name: command.user.name.clone(),
                    target_id: uid.to_string(),
                    target_name,
                    action_type: "ban_permanent".to_string(),
                    reason: reason.to_string(),
                    gravity: None,
                    duration: None,
                })
                .await
            {
                warn!(error = %e, uid = %uid, "Failed to log massban action");
            }
        } else {
            failures += 1;
        }
    }

    let embed = danger_embed(format!("Mass Ban — {} utilisateurs", user_ids.len()))
        .field("Moderateur", format!("<@{}>", command.user.id), true)
        .field("Reussi", success.to_string(), true)
        .field("Echoue", failures.to_string(), true)
        .field("Immunises", immune.to_string(), true)
        .field("Raison", reason, false)
        .timestamp(serenity::model::Timestamp::now())
        .footer(CreateEmbedFooter::new("Moderation | Sentinel"));

    let followup = serenity::builder::CreateInteractionResponseFollowup::new()
        .content(format!(
            "âœ… Mass ban termine : {success}/{} utilisateurs.",
            user_ids.len()
        ))
        .ephemeral(true);
    if let Err(e) = command.create_followup(&ctx.http, followup).await {
        warn!(error = %e, "Failed to send mass ban followup");
    }

    super::log_to_channel(ctx, &guild_id.to_string(), embed).await;

    // BUG #4 : une seule card recapitulative (pas une par membre).
    crate::shared::discord_helpers::post_sanction_summary_card(
        ctx,
        &guild_id.to_string(),
        crate::shared::discord_helpers::SanctionKind::Ban,
        success,
        &command.user.name,
        reason,
    )
    .await;

    info!(
        moderator = %command.user.name,
        success, failures,
        total = user_ids.len(),
        "Mass ban execute"
    );
}

// La logique de parsing vit dans le core hexagonal (avec ses tests).
pub use platform_core::sentinel::domain::services::moderation::user_id_parsing::parse_user_ids;
