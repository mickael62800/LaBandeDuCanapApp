//! Handlers des boutons de confirmation / annulation pour les actions risquees
//! (ban ou mute necessitant une double-validation moderateur).

use serenity::all::{
    ComponentInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage,
    Permissions,
};
use serenity::model::id::UserId;
use tracing::warn;

use super::{commands, risk_check};

/// Re-verifie a l'execution que le membre qui clique possede la permission de
/// moderation `required` (les boutons ne sont pas couverts par
/// `default_member_permissions`). Fail-closed : si le membre ou ses
/// permissions ne peuvent pas etre resolus, on refuse. Retourne `true` si le
/// clic est autorise, `false` si une reponse de refus a deja ete envoyee.
async fn ensure_mod_permission(
    ctx: &Context,
    component: &ComponentInteraction,
    required: Permissions,
) -> bool {
    async fn deny(ctx: &Context, component: &ComponentInteraction, msg: &str) {
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(msg)
                .ephemeral(true),
        );
        let _ = component.create_response(&ctx.http, response).await;
    }

    let Some(guild_id) = component.guild_id else {
        deny(ctx, component, "Tu n'as pas la permission.").await;
        return false;
    };
    let member = match guild_id.member(&ctx.http, component.user.id).await {
        Ok(m) => m,
        Err(_) => {
            deny(ctx, component, "Permissions indisponibles, reessaie.").await;
            return false;
        }
    };
    #[allow(deprecated)]
    let perms = match member.permissions(&ctx.cache) {
        Ok(p) => p,
        Err(_) => {
            deny(ctx, component, "Permissions indisponibles, reessaie.").await;
            return false;
        }
    };
    if !perms.contains(required) && !perms.contains(Permissions::ADMINISTRATOR) {
        deny(ctx, component, "Tu n'as pas la permission.").await;
        return false;
    }
    true
}

pub(super) async fn handle_risky_confirm(ctx: &Context, component: &ComponentInteraction) {
    let pending_id = match component
        .data
        .custom_id
        .strip_prefix(risk_check::CONFIRM_PREFIX)
    {
        Some(id) => id.to_string(),
        None => return,
    };

    let pending = {
        let data = ctx.data.read().await;
        let store = match data.get::<risk_check::RiskyPendingKey>() {
            Some(s) => s,
            None => return,
        };
        risk_check::purge_expired(store);
        store.remove(&pending_id).map(|(_, p)| p)
    };

    let pending = match pending {
        Some(p) => p,
        None => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Cette confirmation a expire ou n'est plus disponible.")
                    .ephemeral(true),
            );
            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send risky expired response");
            }
            return;
        }
    };

    // Re-check permission a l'execution : ce bouton declenche un ban/mute
    // reel. On exige la meme permission que la commande correspondante
    // (/ban -> BAN_MEMBERS, /mute -> MODERATE_MEMBERS). Fail-closed.
    let required = match pending.kind {
        risk_check::PendingKind::Ban { .. } => Permissions::BAN_MEMBERS,
        risk_check::PendingKind::Mute { .. } => Permissions::MODERATE_MEMBERS,
    };
    if !ensure_mod_permission(ctx, component, required).await {
        // On remet l'action en attente pour ne pas la perdre a cause d'un
        // clic non autorise.
        let data = ctx.data.read().await;
        if let Some(store) = data.get::<risk_check::RiskyPendingKey>() {
            store.insert(pending_id, pending);
        }
        return;
    }

    let ack = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .content(format!(
                "\u{2705} Execution confirmee pour `{}`.",
                pending.target_name
            ))
            .embeds(vec![])
            .components(vec![]),
    );
    if let Err(e) = component.create_response(&ctx.http, ack).await {
        warn!(error = %e, "Failed to ACK risky confirm");
    }

    let guild_id = match pending.guild_id.parse::<u64>() {
        Ok(id) => serenity::model::id::GuildId::new(id),
        Err(_) => return,
    };
    let target_uid = match pending.target_id.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return,
    };
    let target_user = match UserId::new(target_uid).to_user(&ctx.http).await {
        Ok(u) => u,
        Err(e) => {
            warn!(error = %e, "risky confirm: user fetch failed");
            return;
        }
    };

    match pending.kind {
        risk_check::PendingKind::Ban {
            delete_message_days,
            is_permanent,
        } => {
            commands::ban::execute_ban(
                ctx,
                pending.channel_id.clone(),
                pending.moderator_id.clone(),
                pending.moderator_name.clone(),
                guild_id,
                target_user.id,
                Some(&target_user),
                &pending.reason,
                pending.duration_secs,
                &pending.duration_label,
                is_permanent,
                delete_message_days,
                None,
            )
            .await;
        }
        risk_check::PendingKind::Mute { timeout_secs } => {
            let is_permanent = pending.duration_secs.is_none();
            commands::mute::execute_mute(
                ctx,
                pending.channel_id.clone(),
                pending.moderator_id.clone(),
                pending.moderator_name.clone(),
                guild_id,
                &target_user,
                &pending.reason,
                pending.duration_secs,
                &pending.duration_label,
                is_permanent,
                timeout_secs,
                None,
            )
            .await;
        }
    }
}

pub(super) async fn handle_risky_cancel(ctx: &Context, component: &ComponentInteraction) {
    let pending_id = match component
        .data
        .custom_id
        .strip_prefix(risk_check::CANCEL_PREFIX)
    {
        Some(id) => id.to_string(),
        None => return,
    };

    // Meme classe de risque : seul un moderateur peut annuler une confirmation
    // en attente. Fail-closed.
    if !ensure_mod_permission(ctx, component, Permissions::MODERATE_MEMBERS).await {
        return;
    }

    let removed = {
        let data = ctx.data.read().await;
        if let Some(store) = data.get::<risk_check::RiskyPendingKey>() {
            store.remove(&pending_id).is_some()
        } else {
            false
        }
    };

    let content = if removed {
        "\u{274c} Action annulee."
    } else {
        "Cette confirmation a deja expire."
    };

    let response = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .content(content)
            .embeds(vec![])
            .components(vec![]),
    );
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Failed to send risky cancel response");
    }
}
