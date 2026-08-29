//! Commande `/purge` — nettoyage de salon (messages système d'épinglage, messages du bot ou messages récents).

use std::collections::HashSet;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse,
    GetMessages, MessageId, MessageType, Permissions,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("purge")
        .description("Supprime des messages dans le salon (notifications d'épingles, messages récents)")
        .default_member_permissions(Permissions::MANAGE_MESSAGES)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "nombre",
                "Nombre de messages à analyser / supprimer (défaut : 50, max : 100)",
            )
            .min_int_value(1)
            .max_int_value(100),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "filtre",
                "Type de messages à supprimer (défaut : tous)",
            )
            .add_string_choice("📌 Notifications d'épingles uniquement", "pins")
            .add_string_choice("🤖 Messages du bot uniquement", "bot")
            .add_string_choice("🗑️ Tous les messages", "tous"),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "garder_epingles",
                "Conserver les messages épinglés (cartes de jeu, règlement) - défaut : true",
            ),
        )
}

pub fn is_purge_admin(permissions: Option<Permissions>) -> bool {
    permissions.is_some_and(|p| {
        p.contains(Permissions::MANAGE_MESSAGES)
            || p.contains(Permissions::MANAGE_GUILD)
            || p.contains(Permissions::ADMINISTRATOR)
    })
}

pub async fn handle_command(ctx: &Context, cmd: &CommandInteraction) {
    let autorise = is_purge_admin(cmd.member.as_ref().and_then(|m| m.permissions));
    if !autorise {
        let _ = cmd
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("⛔ Permission « Gérer les messages » requise pour utiliser `/purge`.")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let _ = cmd
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await;

    let nombre = platform_common_bot::discord_helpers::option_i64(&cmd.data.options, "nombre")
        .unwrap_or(50)
        .clamp(1, 100) as u8;

    let filtre = platform_common_bot::discord_helpers::option_str(&cmd.data.options, "filtre")
        .unwrap_or("tous");

    let garder_epingles = platform_common_bot::discord_helpers::option_bool(&cmd.data.options, "garder_epingles")
        .unwrap_or(true);

    let channel_id = cmd.channel_id;

    // Récupère les messages épinglés si l'on doit les préserver
    let pinned_ids: HashSet<MessageId> = if garder_epingles {
        channel_id
            .pins(&ctx.http)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.id)
            .collect()
    } else {
        HashSet::new()
    };

    let bot_id = ctx.cache.current_user().id;

    let messages = match channel_id.messages(&ctx.http, GetMessages::new().limit(nombre)).await {
        Ok(msgs) => msgs,
        Err(e) => {
            let _ = cmd
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content(format!(
                        "❌ Impossible de récupérer les messages du salon : {e}"
                    )),
                )
                .await;
            return;
        }
    };

    let mut to_delete = Vec::new();

    for msg in &messages {
        // Ne jamais supprimer la réponse différée en cours si elle figure dans la liste
        if msg.id == cmd.id.get() {
            continue;
        }

        // Si le message est épinglé et qu'on doit garder les épingles, on le préserve
        if garder_epingles && pinned_ids.contains(&msg.id) {
            continue;
        }

        let is_pin_notification = msg.kind == MessageType::PinsAdd;
        let is_bot_msg = msg.author.id == bot_id || msg.author.bot;

        let should_delete = match filtre {
            "pins" => is_pin_notification,
            "bot" => is_bot_msg || is_pin_notification,
            _ => true, // "tous"
        };

        if should_delete {
            to_delete.push(msg);
        }
    }

    let total = to_delete.len();
    if total == 0 {
        let _ = cmd
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content("ℹ️ Aucun message correspondant au filtre n'a été trouvé à supprimer."),
            )
            .await;
        return;
    }

    let mut deleted_count = 0;
    let now = chrono::Utc::now();
    let limit_14_days = now - chrono::Duration::days(14);

    let (bulk_candidates, individual_candidates): (Vec<_>, Vec<_>) = to_delete.into_iter().partition(|m| {
        // Bulk delete ne fonctionne que pour les messages de moins de 14 jours
        let timestamp = m.timestamp.unix_timestamp();
        timestamp > limit_14_days.timestamp()
    });

    // Bulk delete si possible (>= 2 messages)
    if bulk_candidates.len() >= 2 {
        let ids: Vec<MessageId> = bulk_candidates.iter().map(|m| m.id).collect();
        match channel_id.delete_messages(&ctx.http, &ids).await {
            Ok(_) => {
                deleted_count += ids.len();
            }
            Err(_) => {
                // Repli sur suppression individuelle
                for m in bulk_candidates {
                    if m.delete(&ctx.http).await.is_ok() {
                        deleted_count += 1;
                    }
                }
            }
        }
    } else {
        for m in bulk_candidates {
            if m.delete(&ctx.http).await.is_ok() {
                deleted_count += 1;
            }
        }
    }

    // Messages plus anciens que 14 jours
    for m in individual_candidates {
        if m.delete(&ctx.http).await.is_ok() {
            deleted_count += 1;
        }
    }

    let _ = cmd
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(format!(
                "🧹 **{deleted_count} message(s)** supprimé(s) avec succès !"
            )),
        )
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_purge_admin() {
        assert!(!is_purge_admin(None));
        assert!(!is_purge_admin(Some(Permissions::SEND_MESSAGES)));
        assert!(is_purge_admin(Some(Permissions::MANAGE_MESSAGES)));
        assert!(is_purge_admin(Some(Permissions::MANAGE_GUILD)));
        assert!(is_purge_admin(Some(Permissions::ADMINISTRATOR)));
    }

    #[test]
    fn test_register_purge_command() {
        let cmd = register();
        let val = serde_json::to_value(&cmd).unwrap();
        assert_eq!(val["name"], "purge");
        assert_eq!(val["options"].as_array().unwrap().len(), 3);
    }
}
