use serenity::all::{
    ButtonStyle, CommandInteraction, CommandOptionType, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, User,
};
use serenity::builder::CreateEmbedFooter;
use tracing::{error, info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::embeds::{critical_embed, moderate_embed, success_embed};

use super::api_client::ModerationAction;
use super::risk_check::{
    self, PendingKind, RiskyPending, RiskyPendingKey, CANCEL_PREFIX, CONFIRM_PREFIX,
};
use super::ModerationApiKey;
use crate::shared::discord_helpers::edit_response_text;

pub fn register() -> CreateCommand {
    CreateCommand::new("mute")
        .description("Mute un utilisateur (temporaire ; vide = 28 jours, maximum Discord)")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison du mute")
                .required(true),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::User,
            "user",
            "Utilisateur a mute (ou utilise user_id)",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "user_id",
            "ID de l'utilisateur (alternative au selecteur)",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "duration",
                "Duree en minutes (vide = 28 jours, le maximum autorise par Discord)",
            )
            .min_int_value(1)
            .max_int_value(40320),
        )
}

pub fn register_unmute() -> CreateCommand {
    CreateCommand::new("unmute")
        .description("Retirer le mute d'un utilisateur")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(CreateCommandOption::new(
            CommandOptionType::User,
            "user",
            "Utilisateur a unmute (ou utilise user_id)",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "user_id",
            "ID de l'utilisateur (alternative au selecteur)",
        ))
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::MODERATE_MEMBERS) {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Permission MODERATE_MEMBERS requise pour /mute.")
                        .ephemeral(true),
                ),
            )
            .await;
        warn!(user = %command.user.name, "Tentative /mute sans permission");
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
        warn!(error = %e, cmd = "mute", "Echec defer interaction Discord");
        return;
    }

    let options = &command.data.options;

    let target_id = match super::resolve_target_user_id(command, "user") {
        Some(id) => id,
        None => {
            edit_response_text(ctx, command, "Parametre 'user' manquant.").await;
            return;
        }
    };

    let reason_raw =
        crate::shared::discord_helpers::option_str(options, "reason").unwrap_or("Aucune raison");
    let reason: &str = &reason_raw.chars().take(500).collect::<String>();

    let duration_minutes = crate::shared::discord_helpers::option_i64(options, "duration");

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

    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            edit_response_text(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };

    if guild_id.member(&ctx.http, target.id).await.is_err() {
        edit_response_text(ctx, command, "Membre introuvable sur le serveur.").await;
        return;
    }

    if let Some(role_id) = super::find_immune_role(ctx, guild_id, target.id).await {
        edit_response_text(ctx, command, &super::immunity_message(role_id, "Mute")).await;
        return;
    }

    let guild_config = crate::shared::discord_helpers::guild_config_or_default(
        ctx,
        &guild_id.to_string(),
        crate::modules::moderation::MODULE_BOT_NAME,
    )
    .await;
    // Discord plafonne un timeout a 28 jours : il n'existe pas de mute
    // reellement permanent cote API (le membre serait demute automatiquement).
    // "Sans duree" applique donc le MAXIMUM autorise -- le plus proche de
    // l'intention -- et on l'annonce clairement au lieu de mentir "permanent".
    const DISCORD_MAX_SECS: u64 = 28 * 24 * 3600;
    let max_mute_duration_secs =
        BaseApiClient::config_u64(&guild_config, "max_mute_duration_secs", DISCORD_MAX_SECS);
    let max_allowed_secs = max_mute_duration_secs.min(DISCORD_MAX_SECS);

    let duration_secs = duration_minutes.map(|m| (m as u64) * 60);
    let is_permanent = duration_minutes.is_none();
    let timeout_secs = duration_secs
        .unwrap_or(max_allowed_secs)
        .min(max_allowed_secs);

    let duration_label = if is_permanent {
        format!("{} jours (max Discord)", max_allowed_secs / (24 * 3600))
    } else {
        format!("{}min", duration_minutes.unwrap())
    };

    if let Some(risk_reason) = risk_check::check_target_risk(ctx, guild_id, &target).await {
        defer_with_confirmation(
            ctx,
            command,
            &target,
            reason,
            duration_secs,
            &duration_label,
            timeout_secs,
            &risk_reason,
        )
        .await;
        return;
    }

    execute_mute(
        ctx,
        command.channel_id.to_string(),
        command.user.id.to_string(),
        command.user.name.clone(),
        guild_id,
        &target,
        reason,
        duration_secs,
        &duration_label,
        is_permanent,
        timeout_secs,
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
    timeout_secs: u64,
    risk_reason: &str,
) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return,
    };

    let pending_id = uuid::Uuid::new_v4().to_string();

    let pending = RiskyPending {
        kind: PendingKind::Mute { timeout_secs },
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
             Action demandee : **Mute ({})**\n\
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

    // Interaction deja deferee au debut du handler : on EDITE (sinon
    // "Interaction has already been acknowledged" et le prompt ne s'affiche pas).
    if let Err(e) = command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new()
                .embed(embed)
                .components(vec![row]),
        )
        .await
    {
        warn!(error = %e, "Failed to send risky mute confirmation prompt");
    }

    info!(
        moderator = %command.user.name,
        target = %target.name,
        risk = %risk_reason,
        "Mute deferred pending confirmation"
    );
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_mute(
    ctx: &Context,
    channel_id: String,
    moderator_id: String,
    moderator_name: String,
    guild_id: serenity::model::id::GuildId,
    target: &User,
    reason: &str,
    duration_secs: Option<u64>,
    duration_label: &str,
    is_permanent: bool,
    timeout_secs: u64,
    command: Option<&CommandInteraction>,
) {
    let mut member = match guild_id.member(&ctx.http, target.id).await {
        Ok(m) => m,
        Err(_) => {
            if let Some(cmd) = command {
                edit_response_text(ctx, cmd, "Membre introuvable sur le serveur.").await;
            }
            return;
        }
    };

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + timeout_secs as i64;

    let datetime = time::OffsetDateTime::from_unix_timestamp(ts)
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let timeout = serenity::model::Timestamp::from(datetime);

    let role_mute =
        match crate::modules::moderation::role_mute::apply(ctx, guild_id, target.id, timeout_secs)
            .await
        {
            Ok(crate::modules::moderation::role_mute::ApplyResult::Applied) => true,
            Ok(crate::modules::moderation::role_mute::ApplyResult::AlreadyActive) => {
                if let Some(cmd) = command {
                    edit_response_text(
                        ctx,
                        cmd,
                        "Ce membre porte deja le role de mute : echeance inchangee.",
                    )
                    .await;
                }
                return;
            }
            Ok(crate::modules::moderation::role_mute::ApplyResult::NotConfigured) => false,
            Err(e) => {
                error!(error = %e, "Impossible de mute l'utilisateur via role");
                if let Some(cmd) = command {
                    edit_response_text(ctx, cmd, &format!("Erreur role de mute : {e}")).await;
                }
                return;
            }
        };
    if !role_mute {
        if let Err(e) = member
            .disable_communication_until_datetime(&ctx.http, timeout)
            .await
        {
            error!(error = %e, "Impossible de mute l'utilisateur");
            if let Some(cmd) = command {
                edit_response_text(ctx, cmd, &format!("Erreur Discord : {e}")).await;
            }
            return;
        }
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
        target_id: target.id.to_string(),
        target_name: target.name.clone(),
        action_type: if is_permanent {
            "mute_permanent".to_string()
        } else {
            "mute_temp".to_string()
        },
        reason: reason.to_string(),
        gravity: None,
        duration: duration_secs,
    };

    let action_id = match api.log_action(&action).await {
        Ok(resp) => Some(resp.id),
        Err(e) => {
            error!(error = %e, "Erreur log mute");
            None
        }
    };

    info!(target = %target.name, duration = %duration_label, "Mute applique");

    let guild_name = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| g.name)
        .unwrap_or_else(|_| "le serveur".into());

    // Discord rend `<t:TS:R>` en relatif localise ("dans 28 jours") : le membre
    // sait exactement quand il sera demute automatiquement.
    let unmute_at = format!("<t:{ts}:F> (<t:{ts}:R>)");

    if let Ok(dm) = target.create_dm_channel(&ctx.http).await {
        let dm_embed = moderate_embed(format!("🔇 Mute ({duration_label}) sur **{guild_name}**"))
            .field("Duree", duration_label, true)
            .field("Demute automatique", &unmute_at, true)
            .field("Raison", reason, false);

        let mut dm_msg = CreateMessage::new().embed(dm_embed);
        // Bouton d'appel (si l'action a bien ete journalisee) : guild_id +
        // action_id embarques dans le custom_id pour router vers le bon serveur.
        if let Some(ref aid) = action_id {
            dm_msg = dm_msg.components(vec![super::appeal::build_appeal_button(
                &guild_id.to_string(),
                aid,
            )]);
        }

        if let Err(e) = dm.send_message(&ctx.http, dm_msg).await {
            warn!(error = %e, "Failed to send mute DM to user");
        }
    }

    let mut channel_embed = moderate_embed(format!("🔇 Mute ({duration_label})"))
        .thumbnail(target.face())
        .field("Cible", format!("<@{}>", target.id), true)
        .field("Moderateur", format!("<@{}>", moderator_id), true)
        .field("Duree", duration_label.to_string(), true)
        .field("Demute automatique", &unmute_at, true)
        .field("ID Cible", target.id.to_string(), true)
        .field("Raison", reason, false);
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
                    "✅ Mute applique sur <@{}> ({}). Demute automatique {}.",
                    target.id, duration_label, unmute_at
                )),
            )
            .await
        {
            warn!(error = %e, "Failed to edit mute response");
        }
    }

    super::log_to_channel(ctx, &guild_id.to_string(), channel_embed).await;

    crate::shared::discord_helpers::post_sanction_card(
        ctx,
        &guild_id.to_string(),
        crate::shared::discord_helpers::SanctionKind::Mute,
        target.id.get(),
        Some(&target.name),
        &moderator_name,
        reason,
        Some(duration_label),
    )
    .await;
}

pub async fn handle_unmute(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::MODERATE_MEMBERS) {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Permission MODERATE_MEMBERS requise pour /unmute.")
                        .ephemeral(true),
                ),
            )
            .await;
        warn!(user = %command.user.name, "Tentative /unmute sans permission");
        return;
    }

    // Defer immediat, comme /mute : la suite enchaine plusieurs appels HTTP
    // (fetch membre, timeout, log API, DM) et depasserait la fenetre de 3s.
    // Sans ce defer les `edit_response_text` d'erreur echouaient tous.
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, cmd = "unmute", "Echec defer interaction Discord");
        return;
    }

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

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            edit_response_text(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let mut member = match guild_id.member(&ctx.http, target_id).await {
        Ok(m) => m,
        Err(_) => {
            edit_response_text(ctx, command, "Membre introuvable.").await;
            return;
        }
    };

    match crate::modules::moderation::role_mute::remove(ctx, guild_id, target_id).await {
        Ok(true) => {}
        Ok(false) => {
            if let Err(e) = member.enable_communication(&ctx.http).await {
                error!(error = %e, "Impossible de unmute");
                edit_response_text(ctx, command, &format!("Erreur : {e}")).await;
                return;
            }
        }
        Err(e) => {
            error!(error = %e, "Impossible de retirer le role de mute");
            edit_response_text(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    }

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => {
            tracing::error!("ModerationApiKey manquant");
            return;
        }
    };
    let target = target_id.to_user(&ctx.http).await.ok();
    let target_name = target
        .as_ref()
        .map(|u| u.name.as_str())
        .unwrap_or("inconnu");

    let action = ModerationAction {
        guild_id: guild_id.to_string(),
        channel_id: command.channel_id.to_string(),
        moderator_id: command.user.id.to_string(),
        moderator_name: command.user.name.clone(),
        target_id: target_id.to_string(),
        target_name: target_name.to_string(),
        action_type: "unmute".to_string(),
        reason: "Unmute manuel".to_string(),
        gravity: None,
        duration: None,
    };

    if let Err(e) = api.log_action(&action).await {
        warn!(error = %e, "Failed to log unmute action");
    }

    info!(target = %target_name, "Unmute applique");

    // DM d'information a la personne : son mute a ete leve.
    if let Some(user) = &target {
        let guild_name = guild_id
            .to_partial_guild(&ctx.http)
            .await
            .map(|g| g.name)
            .unwrap_or_else(|_| "le serveur".into());
        if let Ok(dm) = user.create_dm_channel(&ctx.http).await {
            let dm_embed = success_embed(format!("🔊 Ton mute sur **{guild_name}** a ete leve"))
                .description("Tu peux a nouveau ecrire et parler sur le serveur.")
                .timestamp(serenity::model::Timestamp::now());
            if let Err(e) = dm
                .send_message(&ctx.http, CreateMessage::new().embed(dm_embed))
                .await
            {
                warn!(error = %e, "Failed to send unmute DM to user");
            }
        }
    }

    let unmute_embed = success_embed("🔊 Unmute")
        .field("Cible", format!("<@{target_id}>"), true)
        .field("Moderateur", format!("<@{}>", command.user.id), true)
        .timestamp(serenity::model::Timestamp::now())
        .footer(CreateEmbedFooter::new("Moderation | Sentinel"));

    edit_response_text(ctx, command, &format!("✅ <@{target_id}> a ete unmute.")).await;

    super::log_to_channel(ctx, &guild_id.to_string(), unmute_embed).await;
}
