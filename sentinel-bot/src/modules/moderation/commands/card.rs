//! Commande `/card` : permet a un moderateur de creer manuellement une carte
//! de vote (identique a celle de l'automod) quand une detection est passee au
//! travers. La carte est postee dans le salon de review automod et affiche le
//! contexte AVANT et APRES le message cible. Le flux de vote/finalisation est
//! le meme que l'automod (boutons `amv:`/`amf:`, review en base).

use serenity::all::{
    ChannelId, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse, MessageId,
};
use tracing::warn;

const REQUIRED_PERMISSION: serenity::all::Permissions =
    serenity::all::Permissions::MODERATE_MEMBERS;

pub fn register() -> CreateCommand {
    CreateCommand::new("signalement")
        .description(
            "Signaler un message : cree une carte de vote moderateurs (avec contexte avant/apres)",
        )
        .default_member_permissions(REQUIRED_PERMISSION)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "message",
                "Lien du message (clic droit > Copier le lien) ou ID dans ce salon",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "action", "Action suggeree")
                .required(true)
                .add_string_choice("Avertissement", "warn")
                .add_string_choice("Suppression", "delete")
                .add_string_choice("Mute", "mute")
                .add_string_choice("Bannissement", "ban"),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison du signalement")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "context",
                "Nombre de messages avant et apres a afficher (defaut: 5)",
            )
            .min_int_value(0)
            .max_int_value(15),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, REQUIRED_PERMISSION) {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Permission MODERATE_MEMBERS requise pour /signalement.")
                        .ephemeral(true),
                ),
            )
            .await;
        warn!(
            user = %command.user.name,
            user_id = %command.user.id,
            "Tentative /signalement sans permission"
        );
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
        warn!(error = %e, cmd = "signalement", "Echec defer interaction Discord");
        return;
    }

    let opt_str = |name: &str| {
        command
            .data
            .options
            .iter()
            .find(|o| o.name == name)
            .and_then(|o| match &o.value {
                CommandDataOptionValue::String(s) => Some(s.clone()),
                _ => None,
            })
    };

    let message_ref = opt_str("message").unwrap_or_default();
    let action = opt_str("action").unwrap_or_default();
    let reason = opt_str("reason").unwrap_or_default();
    let context_count = command
        .data
        .options
        .iter()
        .find(|o| o.name == "context")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(i) => Some(*i as u8),
            _ => None,
        })
        .unwrap_or(5);

    // Resout le salon + message a partir d'un lien Discord (cross-salon) ou
    // d'un ID brut (salon courant).
    let (channel_id, message_id) = match parse_message_ref(&message_ref, command.channel_id) {
        Some(v) => v,
        None => {
            let _ = command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content(
                        "Reference invalide. Colle le lien du message (clic droit > Copier le lien) ou son ID.",
                    ),
                )
                .await;
            return;
        }
    };

    // Securite : un lien d'un autre serveur n'a pas de sens ici.
    if let (Some(link_guild), Some(cmd_guild)) = (channel_id.1, command.guild_id) {
        if link_guild != cmd_guild.get() {
            let _ = command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("Ce lien pointe vers un autre serveur."),
                )
                .await;
            return;
        }
    }
    let channel_id = channel_id.0;

    let target = match channel_id
        .message(&ctx.http, MessageId::new(message_id))
        .await
    {
        Ok(m) => m,
        Err(_) => {
            let _ = command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new()
                        .content("Message introuvable (lien errone ou message supprime ?)."),
                )
                .await;
            return;
        }
    };

    let result = crate::modules::automod::create_manual_vote_card(
        ctx,
        &target,
        &action,
        &reason,
        context_count,
        &command.user.name,
    )
    .await;

    let reply = match result {
        Ok(()) => "Carte de vote creee dans le salon de review.".to_string(),
        Err(e) => format!("Impossible de creer la carte : {e}"),
    };
    if let Err(e) = command
        .edit_response(&ctx.http, EditInteractionResponse::new().content(reply))
        .await
    {
        warn!(error = %e, cmd = "signalement", "Echec reponse /card");
    }
}

/// Resout une reference de message en `((salon, guild_du_lien), message_id)`.
/// Accepte un lien Discord `.../channels/<guild>/<channel>/<message>`
/// (cross-salon, `guild` renseigne) ou un ID brut (salon courant, guild None).
fn parse_message_ref(
    input: &str,
    fallback_channel: ChannelId,
) -> Option<((ChannelId, Option<u64>), u64)> {
    let s = input.trim();

    // Cas lien : on prend les 3 derniers segments numeriques apres "channels/".
    if let Some(rest) = s.split("/channels/").nth(1) {
        let parts: Vec<u64> = rest
            .split('/')
            .map(|seg| seg.trim())
            .filter(|seg| !seg.is_empty())
            .map(|seg| seg.parse::<u64>())
            .take_while(|r| r.is_ok())
            .filter_map(|r| r.ok())
            .collect();
        if parts.len() >= 3 {
            let guild = parts[0];
            let channel = parts[1];
            let message = parts[2];
            return Some(((ChannelId::new(channel), Some(guild)), message));
        }
        return None;
    }

    // Cas ID brut : message dans le salon courant.
    let message = s.parse::<u64>().ok()?;
    Some(((fallback_channel, None), message))
}
