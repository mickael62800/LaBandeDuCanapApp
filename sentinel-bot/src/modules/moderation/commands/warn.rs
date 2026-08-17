use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
};
use serenity::builder::CreateEmbedFooter;
use tracing::{error, info, warn};

use crate::shared::embeds::{
    danger_embed, gravity_color, gravity_emoji, moderate_embed, sentinel_embed,
};

use super::api_client::ModerationAction;
use super::ModerationApiKey;
use crate::shared::discord_helpers::edit_response_feedback;
use crate::shared::discord_helpers::edit_response_text;

pub fn register() -> CreateCommand {
    CreateCommand::new("warn")
        .description("Avertir un utilisateur")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "gravity",
                "Gravite de l'avertissement",
            )
            .required(true)
            .add_string_choice("Faible", "low")
            .add_string_choice("Moyenne", "medium")
            .add_string_choice("Haute", "high"),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "reason",
                "Raison de l'avertissement",
            )
            .required(true),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::User,
            "user",
            "Utilisateur a avertir (ou utilise user_id)",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "user_id",
            "ID de l'utilisateur (ex. membre parti / banni)",
        ))
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::MODERATE_MEMBERS) {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Permission MODERATE_MEMBERS requise pour /warn.")
                        .ephemeral(true),
                ),
            )
            .await;
        warn!(user = %command.user.name, "Tentative /warn sans permission");
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
        warn!(error = %e, cmd = "warn", "Echec defer interaction Discord");
        return;
    }

    let options = &command.data.options;

    let target_id = match super::resolve_target_user_id(command, "user") {
        Some(id) => id,
        None => {
            edit_response_text(
                ctx,
                command,
                "Indique un membre (`user`) ou un identifiant (`user_id`).",
            )
            .await;
            return;
        }
    };

    let gravity =
        crate::shared::discord_helpers::option_str(options, "gravity").unwrap_or("medium");

    let reason_raw =
        crate::shared::discord_helpers::option_str(options, "reason").unwrap_or("Aucune raison");
    let reason: &str = &reason_raw.chars().take(500).collect::<String>();

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            edit_response_text(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    if let Err(msg) = super::check_hierarchy(ctx, command, guild_id, target_id) {
        edit_response_text(ctx, command, &format!("❌ {msg}")).await;
        return;
    }

    if let Err(msg) =
        super::check_mod_quota(ctx, &guild_id.to_string(), &command.user.id.to_string()).await
    {
        edit_response_text(ctx, command, &msg).await;
        return;
    }

    let target_opt = target_id.to_user(&ctx.http).await.ok();
    let target_name = target_opt
        .as_ref()
        .map(|u| u.name.clone())
        .unwrap_or_else(|| format!("Utilisateur {}", target_id));

    if let Some(role_id) = super::find_immune_role(ctx, guild_id, target_id).await {
        edit_response_text(ctx, command, &super::immunity_message(role_id, "Warn")).await;
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => {
            tracing::error!("ModerationApiKey manquant");
            return;
        }
    };

    let action = ModerationAction {
        guild_id: guild_id.to_string(),
        channel_id: command.channel_id.to_string(),
        moderator_id: command.user.id.to_string(),
        moderator_name: command.user.name.clone(),
        target_id: target_id.to_string(),
        target_name: target_name.clone(),
        action_type: "warn".to_string(),
        reason: reason.to_string(),
        gravity: Some(gravity.to_string()),
        duration: None,
    };

    match api.log_action(&action).await {
        Ok(resp) => {
            info!(
                action_id = %resp.id,
                target = %target_name,
                gravity = gravity,
                strikes = ?resp.strikes_count,
                escalation = ?resp.escalation_action,
                "Warn enregistre"
            );

            let guild_name = guild_id
                .to_partial_guild(&ctx.http)
                .await
                .map(|g| g.name)
                .unwrap_or_else(|_| "le serveur".into());

            if let Some(ref u) = target_opt {
                if let Ok(dm) = u.create_dm_channel(&ctx.http).await {
                    let dm_embed = sentinel_embed(
                        format!(
                            "{} Avertissement sur **{guild_name}**",
                            gravity_emoji(gravity)
                        ),
                        gravity_color(gravity),
                    )
                    .field("Gravite", gravity, true)
                    .field("Raison", reason, false);

                    let appeal_row =
                        super::appeal::build_appeal_button(&guild_id.to_string(), &resp.id);

                    if let Err(e) = dm
                        .send_message(
                            &ctx.http,
                            CreateMessage::new()
                                .embed(dm_embed)
                                .components(vec![appeal_row]),
                        )
                        .await
                    {
                        warn!(error = %e, "Failed to send warn DM to user");
                    }
                }
            }

            let strikes_label = resp
                .strikes_count
                .map(|c| format!(" — Strike {c}"))
                .unwrap_or_default();
            let mut channel_embed = sentinel_embed(
                format!("{} Warn ({gravity}){strikes_label}", gravity_emoji(gravity)),
                gravity_color(gravity),
            )
            .field("Cible", format!("<@{}>", target_id), true)
            .field("Moderateur", format!("<@{}>", command.user.id), true)
            .field("Gravite", gravity, true)
            .field("ID Cible", target_id.to_string(), true)
            .field("Salon", format!("<#{}>", command.channel_id), true)
            .field(
                "Strikes",
                resp.strikes_count
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "—".to_string()),
                true,
            )
            .field("Raison", reason, false)
            .timestamp(serenity::model::Timestamp::now())
            .footer(CreateEmbedFooter::new("Moderation | Sentinel"));

            if let Some(ref u) = target_opt {
                channel_embed = channel_embed.thumbnail(u.face());
            }

            if let Err(e) = command
                .edit_response(
                    &ctx.http,
                    serenity::builder::EditInteractionResponse::new()
                        .content(format!("✅ Avertissement envoye a <@{}>.", target_id)),
                )
                .await
            {
                warn!(error = %e, "Failed to edit warn response");
            }

            super::log_to_channel(ctx, &guild_id.to_string(), channel_embed).await;

            crate::shared::discord_helpers::post_sanction_card(
                ctx,
                &guild_id.to_string(),
                crate::shared::discord_helpers::SanctionKind::Warn,
                target_id.get(),
                Some(&target_name),
                &command.user.name,
                reason,
                None,
            )
            .await;

            if let Some(ref esc_action) = resp.escalation_action {
                let mut member = match guild_id.member(&ctx.http, target_id).await {
                    Ok(m) => m,
                    Err(_) => return,
                };
                match esc_action.as_str() {
                    "mute" => {
                        let secs = resp.escalation_duration.unwrap_or(600);
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64
                            + secs as i64;
                        let datetime = time::OffsetDateTime::from_unix_timestamp(ts)
                            .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
                        let timeout = serenity::model::Timestamp::from(datetime);
                        if let Err(e) = member
                            .disable_communication_until_datetime(&ctx.http, timeout)
                            .await
                        {
                            warn!(error = %e, "Escalation mute echouee");
                        } else {
                            let mut esc_embed = moderate_embed(format!(
                                "🔇 Mute auto (escalation — {} strikes)",
                                resp.strikes_count.unwrap_or(0)
                            ))
                            .field("Cible", format!("<@{}>", target_id), true)
                            .field("ID Cible", target_id.to_string(), true)
                            .field("Duree", format!("{}min", secs / 60), true)
                            .field(
                                "Declencheur",
                                format!("/warn par <@{}>", command.user.id),
                                false,
                            )
                            .timestamp(serenity::model::Timestamp::now())
                            .footer(CreateEmbedFooter::new("Moderation | Sentinel"));

                            if let Some(ref u) = target_opt {
                                esc_embed = esc_embed.thumbnail(u.face());
                            }

                            super::log_to_channel(ctx, &guild_id.to_string(), esc_embed).await;

                            crate::shared::discord_helpers::post_sanction_card(
                                ctx,
                                &guild_id.to_string(),
                                crate::shared::discord_helpers::SanctionKind::Mute,
                                target_id.get(),
                                Some(&target_name),
                                "Escalade auto",
                                &format!(
                                    "Escalade auto: {} strikes",
                                    resp.strikes_count.unwrap_or(0)
                                ),
                                Some(&format!("{}min", secs / 60)),
                            )
                            .await;

                            let esc_log = ModerationAction {
                                guild_id: guild_id.to_string(),
                                channel_id: command.channel_id.to_string(),
                                moderator_id: command.user.id.to_string(),
                                moderator_name: command.user.name.clone(),
                                target_id: target_id.to_string(),
                                target_name: target_name.clone(),
                                action_type: "mute_temp".to_string(),
                                reason: format!(
                                    "Escalade auto: {} strikes",
                                    resp.strikes_count.unwrap_or(0)
                                ),
                                gravity: None,
                                duration: Some(secs),
                            };
                            if let Err(e) = api.log_action_no_strike(&esc_log).await {
                                warn!(error = %e, "Echec journalisation escalation mute");
                            }
                        }
                    }
                    "ban" => {
                        let mut esc_embed = danger_embed(format!(
                            "🔨 Ban auto (escalation — {} strikes)",
                            resp.strikes_count.unwrap_or(0)
                        ))
                        .field("Cible", format!("<@{}>", target_id), true)
                        .field("ID Cible", target_id.to_string(), true)
                        .field(
                            "Declencheur",
                            format!("/warn par <@{}>", command.user.id),
                            false,
                        )
                        .timestamp(serenity::model::Timestamp::now())
                        .footer(CreateEmbedFooter::new("Moderation | Sentinel"));

                        if let Some(ref u) = target_opt {
                            esc_embed = esc_embed.thumbnail(u.face());
                        }

                        super::log_to_channel(ctx, &guild_id.to_string(), esc_embed).await;
                        match guild_id
                            .ban_with_reason(&ctx.http, target_id, 7, reason)
                            .await
                        {
                            Err(e) => {
                                warn!(error = %e, "Escalation ban echouee");
                            }
                            Ok(()) => {
                                let esc_duration_label = resp
                                    .escalation_duration
                                    .map(|secs| format!("{}h", secs / 3600));
                                crate::shared::discord_helpers::post_sanction_card(
                                    ctx,
                                    &guild_id.to_string(),
                                    crate::shared::discord_helpers::SanctionKind::Ban,
                                    target_id.get(),
                                    Some(&target_name),
                                    "Escalade auto",
                                    &format!(
                                        "Escalade auto: {} strikes",
                                        resp.strikes_count.unwrap_or(0)
                                    ),
                                    esc_duration_label.as_deref().or(Some("permanent")),
                                )
                                .await;

                                // BUG #3 — journaliser l'action d'escalade sans
                                // rejouer de strike. Si l'escalade definit une
                                // duree, c'est un ban_temp : le record d'expiration
                                // ainsi cree declenchera l'auto-unban (BUG #1).
                                let (action_type, duration) = match resp.escalation_duration {
                                    Some(secs) => ("ban_temp".to_string(), Some(secs)),
                                    None => ("ban_permanent".to_string(), None),
                                };
                                let esc_log = ModerationAction {
                                    guild_id: guild_id.to_string(),
                                    channel_id: command.channel_id.to_string(),
                                    moderator_id: command.user.id.to_string(),
                                    moderator_name: command.user.name.clone(),
                                    target_id: target_id.to_string(),
                                    target_name: target_name.clone(),
                                    action_type,
                                    reason: format!(
                                        "Escalade auto: {} strikes",
                                        resp.strikes_count.unwrap_or(0)
                                    ),
                                    gravity: None,
                                    duration,
                                };
                                if let Err(e) = api.log_action_no_strike(&esc_log).await {
                                    warn!(error = %e, "Echec journalisation escalation ban");
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Err(e) => {
            error!(error = %e, "Erreur log warn");
            edit_response_feedback(ctx, command, "⚠️ Impossible d'enregistrer l'avertissement pour le moment, reessaye dans un instant.").await;
        }
    }
}
