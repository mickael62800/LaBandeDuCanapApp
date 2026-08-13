use serenity::builder::{
    CreateActionRow, CreateButton, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
};
use serenity::model::application::{
    ButtonStyle, ComponentInteraction, ComponentInteractionDataKind,
};
use serenity::model::id::{ChannelId, UserId};
use serenity::model::Permissions;
use serenity::prelude::*;
use tracing::{error, info, warn};

use super::api_client::{AddWhitelistRequest, ApiClient, BanFromChannelRequest};

/// Handle access control interactions: invite, kick, ban.
pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();

    match custom_id {
        "select_invite" => handle_invite(ctx, component).await,
        "btn_kick" => handle_kick_menu(ctx, component).await,
        "select_kick" => handle_kick_select(ctx, component).await,
        "btn_ban" => handle_ban_menu(ctx, component).await,
        "select_ban" => handle_ban_select(ctx, component).await,
        other if other.starts_with("ban_duration_") => handle_ban_duration(ctx, component).await,
        _ => {
            warn!(custom_id = %custom_id, "Access control interaction inconnue");
        }
    }
}

// ── Invite ──

async fn handle_invite(ctx: &Context, component: &ComponentInteraction) {
    super::defer_ephemeral(ctx, component).await;
    let Some((voice_channel_id, ch)) = super::require_admin_deferred(ctx, component).await else {
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();

    let selected_users = match &component.data.kind {
        ComponentInteractionDataKind::UserSelect { values } => values.clone(),
        _ => {
            super::respond_followup_ephemeral(
                ctx,
                component,
                "❌ Interaction invalide, relance l'action depuis le panneau du salon.",
            )
            .await;
            return;
        }
    };

    if selected_users.is_empty() {
        super::respond_followup_ephemeral(ctx, component, "Aucun utilisateur selectionne.").await;
        return;
    }

    let mut invited: Vec<UserId> = Vec::new();
    let mut failed: Vec<UserId> = Vec::new();

    for target_id in &selected_users {
        let target_id = *target_id;

        let overwrite = serenity::model::channel::PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL | Permissions::CONNECT | Permissions::SPEAK,
            deny: Permissions::empty(),
            kind: serenity::model::channel::PermissionOverwriteType::Member(target_id),
        };
        if let Err(e) = voice_channel_id
            .create_permission(&ctx.http, overwrite)
            .await
        {
            error!(error = %e, target = %target_id, "Erreur permission invite");
            failed.push(target_id);
            continue;
        }

        if let Some(ref text_id_str) = ch.text_channel_id {
            if let Ok(text_id) = text_id_str.parse::<u64>() {
                let text_overwrite = serenity::model::channel::PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL
                        | Permissions::SEND_MESSAGES
                        | Permissions::READ_MESSAGE_HISTORY,
                    deny: Permissions::empty(),
                    kind: serenity::model::channel::PermissionOverwriteType::Member(target_id),
                };
                if let Err(e) = ChannelId::new(text_id)
                    .create_permission(&ctx.http, text_overwrite)
                    .await
                {
                    tracing::warn!(error = %e, "failed to grant invite permission on text channel");
                }
            }
        }

        if let Some(ref members_id_str) = ch.members_channel_id {
            if let Ok(members_id) = members_id_str.parse::<u64>() {
                let members_overwrite = serenity::model::channel::PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL
                        | Permissions::SEND_MESSAGES
                        | Permissions::READ_MESSAGE_HISTORY,
                    deny: Permissions::empty(),
                    kind: serenity::model::channel::PermissionOverwriteType::Member(target_id),
                };
                if let Err(e) = ChannelId::new(members_id)
                    .create_permission(&ctx.http, members_overwrite)
                    .await
                {
                    tracing::warn!(error = %e, "failed to grant invite permission on members channel");
                }
            }
        }

        let target_name = target_id
            .to_user(&ctx.http)
            .await
            .map(|u| u.name.clone())
            .unwrap_or_else(|_| target_id.get().to_string());

        let request = AddWhitelistRequest {
            guild_id: guild_id.get().to_string(),
            owner_id: ch.owner_id.clone(),
            target_id: target_id.get().to_string(),
            target_name: target_name.clone(),
        };

        {
            let data = ctx.data.read().await;
            let Some(api) = ApiClient::from_data(&data) else {
                error!("ApiClient ou GrpcClient manquants dans TypeMap");
                return;
            };
            if let Err(e) = api.add_to_whitelist(&request).await {
                warn!(error = %e, "Erreur API whitelist");
            }
        }

        invited.push(target_id);
        info!(voice = %voice_channel_id, target = %target_id, "Utilisateur invite");
    }

    let mut message = if invited.is_empty() {
        "❌ Aucun membre n'a pu etre invite (le bot manque peut-etre la permission de gerer ce salon).".to_string()
    } else {
        let mentions = invited
            .iter()
            .map(|id| format!("<@{id}>"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{mentions} a ete invite dans le salon.")
    };
    if !failed.is_empty() && !invited.is_empty() {
        let mentions = failed
            .iter()
            .map(|id| format!("<@{id}>"))
            .collect::<Vec<_>>()
            .join(", ");
        message.push_str(&format!("\n⚠️ Echec pour : {mentions}."));
    }

    super::respond_followup_ephemeral(ctx, component, &message).await;
}

// ── Kick ──

async fn handle_kick_menu(ctx: &Context, component: &ComponentInteraction) {
    let Some((voice_channel_id, _ch)) = super::require_admin(ctx, component).await else {
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();
    let owner_id = component.user.id;

    let members = get_voice_members(ctx, guild_id, voice_channel_id, Some(owner_id)).await;

    if members.is_empty() {
        super::respond_ephemeral(
            ctx,
            component,
            "Aucun membre a expulser dans le salon vocal.",
        )
        .await;
        return;
    }

    let options: Vec<CreateSelectMenuOption> = members
        .iter()
        .map(|(id, name)| CreateSelectMenuOption::new(name, id.get().to_string()))
        .collect();

    let select = CreateSelectMenu::new("select_kick", CreateSelectMenuKind::String { options })
        .placeholder("Choisissez un membre a expulser");

    let row = CreateActionRow::SelectMenu(select);

    let msg = CreateInteractionResponseMessage::new()
        .content("Qui souhaitez-vous expulser ?")
        .components(vec![row])
        .ephemeral(true);

    let response = CreateInteractionResponse::Message(msg);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur envoi menu kick");
    }
}

async fn handle_kick_select(ctx: &Context, component: &ComponentInteraction) {
    super::defer_ephemeral(ctx, component).await;
    let Some((_voice_channel_id_check, ch)) = super::require_admin_deferred(ctx, component).await
    else {
        return;
    };

    let text_channel_id = component.channel_id;

    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await
    {
        vc
    } else {
        super::respond_followup_ephemeral(
            ctx,
            component,
            "Impossible de trouver le salon vocal associe.",
        )
        .await;
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();

    let selected_value = match &component.data.kind {
        serenity::model::application::ComponentInteractionDataKind::StringSelect { values } => {
            match values.first() {
                Some(v) => v.clone(),
                None => {
                    super::respond_followup_ephemeral(ctx, component, "Aucun membre selectionne.")
                        .await;
                    return;
                }
            }
        }
        _ => {
            super::respond_followup_ephemeral(ctx, component, "Selection invalide.").await;
            return;
        }
    };

    let target_id: u64 = match selected_value.parse() {
        Ok(id) => id,
        Err(_) => {
            super::respond_followup_ephemeral(ctx, component, "Selection invalide.").await;
            return;
        }
    };

    let target_user_id = UserId::new(target_id);

    // #2 : on ne peut pas expulser le PROPRIETAIRE du salon (un co-admin qui a
    // acces au menu ne doit pas pouvoir virer l'owner).
    if ch.owner_id == target_id.to_string() {
        super::respond_followup_ephemeral(
            ctx,
            component,
            "Tu ne peux pas expulser le proprietaire du salon.",
        )
        .await;
        return;
    }
    // #4 : la cible doit REELLEMENT etre dans CE salon vocal (un custom_id/select
    // forge ne doit pas permettre de deconnecter un membre d'un autre salon du
    // serveur).
    let target_here = ctx
        .cache
        .guild(guild_id)
        .map(|g| {
            g.voice_states
                .get(&target_user_id)
                .and_then(|vs| vs.channel_id)
                == Some(voice_channel_id)
        })
        .unwrap_or(false);
    if !target_here {
        super::respond_followup_ephemeral(ctx, component, "Ce membre n'est pas dans ton salon.")
            .await;
        return;
    }

    match guild_id.disconnect_member(&ctx.http, target_user_id).await {
        Ok(_) => {
            info!(voice = %voice_channel_id, target = %target_user_id, "Membre expulse");
        }
        Err(e) => {
            error!(error = %e, "Erreur disconnect membre");
            super::respond_followup_ephemeral(ctx, component, "Erreur lors de l'expulsion.").await;
            return;
        }
    }

    super::respond_followup_ephemeral(
        ctx,
        component,
        &format!("<@{target_id}> a ete expulse du salon."),
    )
    .await;
}

// ── Ban ──

async fn handle_ban_menu(ctx: &Context, component: &ComponentInteraction) {
    let Some((voice_channel_id, _ch)) = super::require_admin(ctx, component).await else {
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();
    let owner_id = component.user.id;

    let members = get_voice_members(ctx, guild_id, voice_channel_id, Some(owner_id)).await;

    if members.is_empty() {
        super::respond_ephemeral(ctx, component, "Aucun membre a bannir dans le salon vocal.")
            .await;
        return;
    }

    let options: Vec<CreateSelectMenuOption> = members
        .iter()
        .map(|(id, name)| CreateSelectMenuOption::new(name, id.get().to_string()))
        .collect();

    let select = CreateSelectMenu::new("select_ban", CreateSelectMenuKind::String { options })
        .placeholder("Choisissez un membre a bannir");

    let row = CreateActionRow::SelectMenu(select);

    let msg = CreateInteractionResponseMessage::new()
        .content("Qui souhaitez-vous bannir ?")
        .components(vec![row])
        .ephemeral(true);

    let response = CreateInteractionResponse::Message(msg);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur envoi menu ban");
    }
}

async fn handle_ban_select(ctx: &Context, component: &ComponentInteraction) {
    let selected_value = match &component.data.kind {
        serenity::model::application::ComponentInteractionDataKind::StringSelect { values } => {
            match values.first() {
                Some(v) => v.clone(),
                None => {
                    super::respond_ephemeral(ctx, component, "Aucun membre selectionne.").await;
                    return;
                }
            }
        }
        _ => {
            super::respond_ephemeral(ctx, component, "Selection invalide.").await;
            return;
        }
    };

    // Durees des presets de voice-ban : reglables par serveur via la cle CSV
    // `voice_ban_preset_secs` (3 valeurs). Fallback aux defauts si absente/malformee.
    let presets = ban_duration_presets(ctx, component).await;
    let row = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("ban_duration_{selected_value}_{}", presets[0]))
            .label(format_ban_duration(presets[0]))
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("ban_duration_{selected_value}_{}", presets[1]))
            .label(format_ban_duration(presets[1]))
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("ban_duration_{selected_value}_{}", presets[2]))
            .label(format_ban_duration(presets[2]))
            .style(ButtonStyle::Danger),
        CreateButton::new(format!("ban_duration_{selected_value}_0"))
            .label("Permanent")
            .style(ButtonStyle::Danger),
    ]);

    let msg = CreateInteractionResponseMessage::new()
        .content(format!("Duree du ban pour <@{selected_value}> ?"))
        .components(vec![row])
        .ephemeral(true);

    let response = CreateInteractionResponse::Message(msg);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur envoi menu duree ban");
    }
}

/// Lit les 3 durees (secondes) des boutons de voice-ban depuis la config
/// voice-bot (`voice_ban_preset_secs`, CSV de 3 entiers). Retombe sur les
/// defauts historiques [300, 3600, 86400] si la cle est absente ou malformee.
async fn ban_duration_presets(ctx: &Context, component: &ComponentInteraction) -> [u64; 3] {
    const DEFAULTS: [u64; 3] = [300, 3600, 86400];
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return DEFAULTS,
    };
    let cfg = {
        let data = ctx.data.read().await;
        match data.get::<crate::shared::heartbeat::ApiClientKey>() {
            Some(base) => base
                .get_guild_config_for(
                    &guild_id.to_string(),
                    crate::modules::voice::MODULE_BOT_NAME,
                )
                .await
                .unwrap_or_default(),
            None => return DEFAULTS,
        }
    };
    let raw =
        crate::shared::api_client::BaseApiClient::config_or(&cfg, "voice_ban_preset_secs", "");
    let parsed =
        platform_core::sentinel::domain::entities::system::config_parsers::parse_u64_csv(&raw);
    if parsed.len() == 3 && parsed.iter().all(|&v| v > 0) {
        [parsed[0], parsed[1], parsed[2]]
    } else {
        DEFAULTS
    }
}

/// Formate une duree (secondes) en libelle court pour un bouton (ex: "5 min").
fn format_ban_duration(secs: u64) -> String {
    if secs.is_multiple_of(86400) {
        format!("{} jour(s)", secs / 86400)
    } else if secs.is_multiple_of(3600) {
        format!("{} heure(s)", secs / 3600)
    } else if secs.is_multiple_of(60) {
        format!("{} min", secs / 60)
    } else {
        format!("{secs} s")
    }
}

async fn handle_ban_duration(ctx: &Context, component: &ComponentInteraction) {
    super::defer_ephemeral(ctx, component).await;
    let Some((_voice_channel_id_check, ch)) = super::require_admin_deferred(ctx, component).await
    else {
        return;
    };

    let custom_id = component.data.custom_id.as_str();

    let parts: Vec<&str> = custom_id
        .strip_prefix("ban_duration_")
        .unwrap_or("")
        .rsplitn(2, '_')
        .collect();
    if parts.len() < 2 {
        super::respond_followup_ephemeral(ctx, component, "Format invalide.").await;
        return;
    }

    let duration_secs: i64 = parts[0].parse().unwrap_or(0);
    let target_id_str = parts[1];
    let target_id: u64 = match target_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            super::respond_followup_ephemeral(ctx, component, "ID utilisateur invalide.").await;
            return;
        }
    };

    let target_user_id = UserId::new(target_id);
    let text_channel_id = component.channel_id;

    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await
    {
        vc
    } else {
        super::respond_followup_ephemeral(
            ctx,
            component,
            "Impossible de trouver le salon vocal associe.",
        )
        .await;
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();

    // #2 : on ne bannit pas le PROPRIETAIRE du salon.
    if ch.owner_id == target_id.to_string() {
        super::respond_followup_ephemeral(
            ctx,
            component,
            "Tu ne peux pas bannir le proprietaire du salon.",
        )
        .await;
        return;
    }
    // #4 : la cible doit REELLEMENT etre dans CE salon (empeche un custom_id
    // forge de deconnecter/bannir un membre d'un autre salon du serveur).
    let target_here = ctx
        .cache
        .guild(guild_id)
        .map(|g| {
            g.voice_states
                .get(&target_user_id)
                .and_then(|vs| vs.channel_id)
                == Some(voice_channel_id)
        })
        .unwrap_or(false);
    if !target_here {
        super::respond_followup_ephemeral(ctx, component, "Ce membre n'est pas dans ton salon.")
            .await;
        return;
    }

    if let Err(e) = guild_id.disconnect_member(&ctx.http, target_user_id).await {
        tracing::warn!(error = %e, "failed to disconnect banned member");
    }

    let overwrite = serenity::model::channel::PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::VIEW_CHANNEL | Permissions::CONNECT,
        kind: serenity::model::channel::PermissionOverwriteType::Member(target_user_id),
    };
    if let Err(e) = voice_channel_id
        .create_permission(&ctx.http, overwrite)
        .await
    {
        tracing::warn!(error = %e, "failed to set ban permission on voice channel");
    }

    let target_name = target_user_id
        .to_user(&ctx.http)
        .await
        .map(|u| u.name.clone())
        .unwrap_or_else(|_| target_id.to_string());

    let ban_request = BanFromChannelRequest {
        user_id: target_id.to_string(),
        user_name: target_name.clone(),
        banned_by: component.user.id.get().to_string(),
        reason: None,
        duration_secs: if duration_secs == 0 {
            None
        } else {
            Some(duration_secs)
        },
    };

    {
        let data = ctx.data.read().await;
        let Some(api) = ApiClient::from_data(&data) else {
            error!("ApiClient ou GrpcClient manquants dans TypeMap");
            return;
        };
        if let Err(e) = api
            .ban_user(&voice_channel_id.get().to_string(), &ban_request)
            .await
        {
            error!(error = %e, "Erreur API ban");
        }
    }

    let duration_text = match duration_secs {
        0 => "permanent".to_string(),
        300 => "5 minutes".to_string(),
        3600 => "1 heure".to_string(),
        86400 => "24 heures".to_string(),
        s => format!("{s} secondes"),
    };

    super::respond_followup_ephemeral(
        ctx,
        component,
        &format!("<@{target_id}> a ete banni du salon ({duration_text})."),
    )
    .await;

    info!(
        voice = %voice_channel_id,
        target = %target_user_id,
        duration = duration_secs,
        "Utilisateur banni"
    );
}

// ── Helpers ──

async fn get_voice_members(
    ctx: &Context,
    guild_id: serenity::model::id::GuildId,
    voice_channel_id: ChannelId,
    exclude: Option<UserId>,
) -> Vec<(UserId, String)> {
    let mut members = Vec::new();

    let guild = match ctx.cache.guild(guild_id) {
        Some(g) => g.clone(),
        None => return members,
    };

    for (user_id, voice_state) in &guild.voice_states {
        if voice_state.channel_id == Some(voice_channel_id) {
            if let Some(exc) = exclude {
                if *user_id == exc {
                    continue;
                }
            }

            let name = user_id
                .to_user(&ctx.http)
                .await
                .map(|u| u.name.clone())
                .unwrap_or_else(|_| user_id.get().to_string());

            members.push((*user_id, name));
        }
    }

    members
}
