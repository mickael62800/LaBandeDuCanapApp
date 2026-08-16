use serenity::all::{
    ButtonStyle, CommandInteraction, CommandOptionType, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, User,
};
use serenity::builder::CreateEmbedFooter;
use tracing::{error, info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::embeds::{critical_embed, success_embed};

use super::api_client::ModerationAction;
use super::risk_check::{
    self, PendingKind, RiskyPending, RiskyPendingKey, CANCEL_PREFIX, CONFIRM_PREFIX,
};
use super::ModerationApiKey;
use crate::shared::discord_helpers::edit_response_text;

pub fn register() -> CreateCommand {
    CreateCommand::new("ban")
        .description("Bannir un utilisateur (permanent ou temporaire)")
        .default_member_permissions(serenity::all::Permissions::BAN_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison du ban")
                .required(true),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::User,
            "user",
            "Utilisateur a bannir (ou utilise user_id)",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "user_id",
            "ID de l'utilisateur (ex. deja parti du serveur)",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "duration",
                "Duree en heures (vide = permanent)",
            )
            .min_int_value(1)
            .max_int_value(672), // 28 jours (plafond Discord)
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "purge",
                "Supprimer les messages recents du banni (vide = reglage du serveur)",
            )
            .add_int_choice("Aucun message", 0)
            .add_int_choice("1 jour", 1)
            .add_int_choice("3 jours", 3)
            .add_int_choice("7 jours", 7),
        )
}

pub fn register_unban() -> CreateCommand {
    CreateCommand::new("unban")
        .description("Debannir un utilisateur")
        .default_member_permissions(serenity::all::Permissions::BAN_MEMBERS)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "user_id",
                "ID de l'utilisateur a debannir",
            )
            .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::BAN_MEMBERS) {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Permission BAN_MEMBERS requise pour /ban.")
                        .ephemeral(true),
                ),
            )
            .await;
        warn!(user = %command.user.name, "Tentative /ban sans permission");
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
        warn!(error = %e, cmd = "ban", "Echec defer interaction Discord");
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

    let reason_raw =
        crate::shared::discord_helpers::option_str(options, "reason").unwrap_or("Aucune raison");
    let reason: &str = &reason_raw.chars().take(500).collect::<String>();

    let duration_hours = crate::shared::discord_helpers::option_i64(options, "duration");

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            edit_response_text(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    // Hierarchie : pas de sanction sur soi / le bot / l'owner / un rang >= au sien.
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

    let target = target_id.to_user(&ctx.http).await.ok();

    if let Some(role_id) = super::find_immune_role(ctx, guild_id, target_id).await {
        edit_response_text(ctx, command, &super::immunity_message(role_id, "Ban")).await;
        return;
    }

    let is_permanent = duration_hours.is_none();
    // Defense en profondeur (l'option est deja bornee 1..=672) : rejette les
    // valeurs non positives et sature la multiplication.
    let duration_secs = duration_hours
        .and_then(|h| u64::try_from(h).ok())
        .filter(|&h| h > 0)
        .map(|h| h.saturating_mul(3600));
    let duration_label = if is_permanent {
        "permanent".to_string()
    } else {
        format!("{}h", duration_hours.unwrap())
    };

    let guild_config = crate::shared::discord_helpers::guild_config_or_default(
        ctx,
        &guild_id.to_string(),
        crate::modules::moderation::MODULE_BOT_NAME,
    )
    .await;
    // Purge des messages : l'option de commande (0/1/3/7 j, max Discord) prime
    // sur le reglage serveur `ban_delete_message_days` si le modo l'a choisie.
    let purge_opt = crate::shared::discord_helpers::option_i64(options, "purge");
    let ban_delete_message_days = match purge_opt {
        Some(n) => n.clamp(0, 7) as u8,
        None => BaseApiClient::config_u64(&guild_config, "ban_delete_message_days", 7) as u8,
    };

    let mut risk_reason_opt = None;
    if let Some(ref u) = target {
        risk_reason_opt = risk_check::check_target_risk(ctx, guild_id, u).await;
    }

    if let Some(risk_reason) = risk_reason_opt {
        defer_with_confirmation(
            ctx,
            command,
            target.as_ref().unwrap(),
            reason,
            duration_secs,
            &duration_label,
            is_permanent,
            ban_delete_message_days,
            &risk_reason,
        )
        .await;
        return;
    }

    execute_ban(
        ctx,
        command.channel_id.to_string(),
        command.user.id.to_string(),
        command.user.name.clone(),
        guild_id,
        target_id,
        target.as_ref(),
        reason,
        duration_secs,
        &duration_label,
        is_permanent,
        ban_delete_message_days,
        Some(command),
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
async fn defer_with_confirmation(
    ctx: &Context,
    command: &CommandInteraction,
    target: &User,
    reason: &str,
    duration_secs: Option<u64>,
    duration_label: &str,
    is_permanent: bool,
    ban_delete_message_days: u8,
    risk_reason: &str,
) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return,
    };

    let pending_id = uuid::Uuid::new_v4().to_string();

    let pending = RiskyPending {
        kind: PendingKind::Ban {
            delete_message_days: ban_delete_message_days,
            is_permanent,
        },
        guild_id: guild_id.to_string(),
        channel_id: command.channel_id.to_string(),
        target_id: target.id.to_string(),
        target_name: target.name.clone(),
        moderator_id: command.user.id.to_string(),
        moderator_name: command.user.name.clone(),
        reason: reason.to_string(),
        duration_secs,
        duration_label: duration_label.to_string(),
        created_at: std::time::Instant::now(),
    };

    {
        let data = ctx.data.read().await;
        if let Some(store) = data.get::<RiskyPendingKey>() {
            risk_check::purge_expired(store);
            store.insert(pending_id.clone(), pending);
        }
    }

    let embed = critical_embed("\u{26a0}\u{fe0f} Confirmation requise — cible a risque")
        .description(format!(
            "La cible <@{}> (`{}`) presente un risque : **{}**.\n\n\
             Action demandee : **Ban ({})**\n\
             Raison : {}\n\n\
             Confirmer l'execution ?",
            target.id, target.name, risk_reason, duration_label, reason
        ));

    let row = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{CONFIRM_PREFIX}{pending_id}"))
            .label("Confirmer")
            .style(ButtonStyle::Danger),
        CreateButton::new(format!("{CANCEL_PREFIX}{pending_id}"))
            .label("Annuler")
            .style(ButtonStyle::Secondary),
    ]);

    // L'interaction a deja ete deferee (ephemere) au debut du handler : on
    // EDITE la reponse differee au lieu de create_response (sinon Discord
    // renvoie "Interaction has already been acknowledged" et le prompt de
    // confirmation ne s'affiche jamais -> impossible de bannir une cible a risque).
    if let Err(e) = command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new()
                .embed(embed)
                .components(vec![row]),
        )
        .await
    {
        warn!(error = %e, "Failed to send risky confirmation prompt");
    }

    info!(
        moderator = %command.user.name,
        target = %target.name,
        risk = %risk_reason,
        "Ban deferred pending confirmation"
    );
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_ban(
    ctx: &Context,
    channel_id: String,
    moderator_id: String,
    moderator_name: String,
    guild_id: serenity::model::id::GuildId,
    target_id: serenity::model::id::UserId,
    target_opt: Option<&User>,
    reason: &str,
    duration_secs: Option<u64>,
    duration_label: &str,
    is_permanent: bool,
    ban_delete_message_days: u8,
    command: Option<&CommandInteraction>,
) {
    let guild_name = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| g.name)
        .unwrap_or_else(|_| "le serveur".into());

    let target_name = target_opt.map(|u| u.name.clone()).unwrap_or_else(|| format!("Utilisateur {}", target_id));

    // Ouvre le canal DM AVANT le ban (le bot partage encore un serveur avec la
    // cible). L'envoi du message est differe apres la journalisation pour
    // embarquer l'action_id reel dans le bouton d'appel ; un canal DM deja
    // ouvert reste joignable meme apres le ban.
    let dm_channel = if let Some(u) = target_opt {
        u.create_dm_channel(&ctx.http).await.ok()
    } else {
        None
    };

    if let Err(e) = guild_id
        .ban_with_reason(&ctx.http, target_id, ban_delete_message_days, reason)
        .await
    {
        error!(error = %e, "Impossible de bannir");
        if let Some(cmd) = command {
            edit_response_text(ctx, cmd, &format!("Erreur Discord : {e}")).await;
        }
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
        channel_id,
        moderator_id: moderator_id.clone(),
        moderator_name: moderator_name.clone(),
        target_id: target_id.to_string(),
        target_name: target_name.clone(),
        action_type: if is_permanent {
            "ban_permanent".to_string()
        } else {
            "ban_temp".to_string()
        },
        reason: reason.to_string(),
        gravity: None,
        duration: duration_secs,
    };

    let action_id = match api.log_action(&action).await {
        Ok(resp) => Some(resp.id),
        Err(e) => {
            error!(error = %e, "Erreur log ban");
            None
        }
    };

    // DM differe (canal ouvert avant le ban) avec bouton d'appel.
    if let Some(dm) = dm_channel {
        let dm_embed = critical_embed(format!("🔨 Ban ({duration_label}) sur **{guild_name}**"))
            .field("Raison", reason, false);
        let mut dm_msg = CreateMessage::new().embed(dm_embed);
        if let Some(ref aid) = action_id {
            dm_msg = dm_msg.components(vec![super::appeal::build_appeal_button(
                &guild_id.to_string(),
                aid,
            )]);
        }
        if let Err(e) = dm.send_message(&ctx.http, dm_msg).await {
            warn!(error = %e, "Failed to send ban DM to user");
        }
    }

    info!(target = %target_name, duration = %duration_label, "Ban applique");

    let mut channel_embed = critical_embed(format!("🔨 Ban ({duration_label})"))
        .field("Cible", format!("<@{}>", target_id), true)
        .field("Moderateur", format!("<@{}>", moderator_id), true)
        .field("Duree", duration_label.to_string(), true)
        .field("ID Cible", target_id.to_string(), true)
        .field("Raison", reason, false);
    
    if let Some(u) = target_opt {
        channel_embed = channel_embed.thumbnail(u.face());
    }

    if let Some(cmd) = command {
        channel_embed = channel_embed.field("Salon", format!("<#{}>", cmd.channel_id), true);
    }
    let channel_embed = channel_embed
        .timestamp(serenity::model::Timestamp::now())
        .footer(CreateEmbedFooter::new("Moderation | Sentinel"));

    if let Some(cmd) = command {
        if let Err(e) = cmd
            .edit_response(
                &ctx.http,
                serenity::builder::EditInteractionResponse::new().content(format!(
                    "✅ Ban applique sur <@{}> ({}).",
                    target_id, duration_label
                )),
            )
            .await
        {
            warn!(error = %e, "Failed to edit ban response");
        }
    }

    super::log_to_channel(ctx, &guild_id.to_string(), channel_embed).await;

    crate::shared::discord_helpers::post_sanction_card(
        ctx,
        &guild_id.to_string(),
        crate::shared::discord_helpers::SanctionKind::Ban,
        target_id.get(),
        Some(&target_name),
        &moderator_name,
        reason,
        Some(duration_label),
    )
    .await;
}

pub async fn handle_unban(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::BAN_MEMBERS) {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Permission BAN_MEMBERS requise pour /unban.")
                        .ephemeral(true),
                ),
            )
            .await;
        warn!(user = %command.user.name, "Tentative /unban sans permission");
        return;
    }

    let user_id_str =
        crate::shared::discord_helpers::option_str(&command.data.options, "user_id").unwrap_or("0");

    let user_id: u64 = match user_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            edit_response_text(ctx, command, "ID utilisateur invalide.").await;
            return;
        }
    };

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            edit_response_text(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let target_uid = serenity::model::id::UserId::new(user_id);

    if let Err(e) = guild_id.unban(&ctx.http, target_uid).await {
        error!(error = %e, "Impossible de debannir");
        edit_response_text(ctx, command, &format!("Erreur : {e}")).await;
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
        target_id: user_id_str.to_string(),
        target_name: "inconnu".to_string(),
        action_type: "unban".to_string(),
        reason: "Unban manuel".to_string(),
        gravity: None,
        duration: None,
    };

    if let Err(e) = api.log_action(&action).await {
        warn!(error = %e, "Failed to log unban action");
    }

    info!(target_id = user_id_str, "Unban applique");

    let unban_embed = success_embed("✅ Unban")
        .field("Moderateur", format!("<@{}>", command.user.id), true)
        .field("Utilisateur", format!("`{user_id_str}`"), false)
        .timestamp(serenity::model::Timestamp::now())
        .footer(CreateEmbedFooter::new("Moderation | Sentinel"));

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!("✅ Unban applique sur `{user_id_str}`."))
                    .ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Failed to send unban response");
    }

    super::log_to_channel(ctx, &guild_id.to_string(), unban_embed).await;
}
