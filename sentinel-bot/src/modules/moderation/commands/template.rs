use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::warn;

use crate::shared::api_client::BaseApiClient;
use crate::shared::embeds::{info_embed, success_embed};
use crate::shared::heartbeat::ApiClientKey;

use super::reason_templates::{self, ReasonTemplate};
use super::ModerationApiKey;
use crate::shared::discord_helpers::edit_response_text;

const CONFIG_KEY: &str = "reason_templates";
const BOT_NAME: &str = "moderation-bot";
const REQUIRED_PERMISSION: serenity::all::Permissions = serenity::all::Permissions::ADMINISTRATOR;

pub fn register() -> CreateCommand {
    CreateCommand::new("template")
        .description("Gerer les templates de raisons de moderation (senior mods)")
        .default_member_permissions(REQUIRED_PERMISSION)
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "Afficher les templates actuels",
        ))
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "add", "Ajouter un template")
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "label",
                        "Libelle court du template (ex: Spam)",
                    )
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "reason",
                        "Raison a inserer (ex: Messages repetitifs)",
                    )
                    .required(true),
                ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "remove",
                "Supprimer un template par son label",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "label",
                    "Label exact du template a supprimer",
                )
                .required(true),
            ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, REQUIRED_PERMISSION) {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Permission ADMINISTRATOR requise pour /template.")
                        .ephemeral(true),
                ),
            )
            .await;
        warn!(
            user = %command.user.name,
            user_id = %command.user.id,
            "Tentative /template sans permission administrateur"
        );
        return;
    }

    // Defer immediat : les sous-handlers lisent/ecrivent la config via l'API et
    // depasseraient la fenetre de 3s de Discord ; sans acquittement leurs
    // `edit_response_*` echouent silencieusement.
    crate::shared::discord_helpers::defer_ephemeral(ctx, command).await;

    let sub = command.data.options.iter().find_map(|o| match &o.value {
        CommandDataOptionValue::SubCommand(inner) => Some((o.name.as_str(), inner.as_slice())),
        _ => None,
    });

    let (sub_name, sub_opts) = match sub {
        Some(s) => s,
        None => {
            edit_response_text(ctx, command, "Sous-commande manquante.").await;
            return;
        }
    };

    match sub_name {
        "list" => handle_list(ctx, command).await,
        "add" => handle_add(ctx, command, sub_opts).await,
        "remove" => handle_remove(ctx, command, sub_opts).await,
        _ => edit_response_text(ctx, command, "Sous-commande inconnue.").await,
    }
}

async fn handle_list(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            edit_response_text(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let templates = match load_templates(ctx, &guild_id).await {
        Ok(t) => t,
        Err(e) => {
            edit_response_text(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let description = if templates.is_empty() {
        "Aucun template defini. Utilisez `/template add` pour en ajouter.".to_string()
    } else {
        templates
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{}. **{}** — {}", i + 1, t.label, t.reason))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let embed = info_embed(format!(
        "\u{1f4cb} Templates de raisons ({})",
        templates.len()
    ))
    .description(description);

    crate::shared::discord_helpers::edit_response_embed(ctx, command, embed).await;
}

async fn handle_add(
    ctx: &Context,
    command: &CommandInteraction,
    opts: &[serenity::model::application::CommandDataOption],
) {
    let label = opts
        .iter()
        .find(|o| o.name == "label")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("")
        .trim();
    let reason = opts
        .iter()
        .find(|o| o.name == "reason")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("")
        .trim();

    if label.is_empty() || reason.is_empty() {
        edit_response_text(ctx, command, "label et reason requis (non vides).").await;
        return;
    }
    if label.contains('|') || label.contains('\n') {
        edit_response_text(
            ctx,
            command,
            "Le label ne peut pas contenir `|` ni de saut de ligne.",
        )
        .await;
        return;
    }
    if reason.contains('\n') {
        edit_response_text(
            ctx,
            command,
            "La raison ne peut pas contenir de saut de ligne.",
        )
        .await;
        return;
    }

    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            edit_response_text(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let mut templates = match load_templates(ctx, &guild_id).await {
        Ok(t) => t,
        Err(e) => {
            edit_response_text(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };

    if templates
        .iter()
        .any(|t| t.label.eq_ignore_ascii_case(label))
    {
        edit_response_text(
            ctx,
            command,
            "Un template avec ce label existe deja. Utilisez `/template remove` d'abord.",
        )
        .await;
        return;
    }

    templates.push(ReasonTemplate {
        label: label.to_string(),
        reason: reason.to_string(),
    });

    if let Err(e) = save_templates(ctx, &guild_id, &templates).await {
        edit_response_text(ctx, command, &format!("Erreur : {e}")).await;
        return;
    }

    let embed = success_embed("\u{2705} Template ajoute")
        .field("Label", label, true)
        .field("Raison", reason, false)
        .field("Total", templates.len().to_string(), true);

    crate::shared::discord_helpers::edit_response_embed(ctx, command, embed).await;
}

async fn handle_remove(
    ctx: &Context,
    command: &CommandInteraction,
    opts: &[serenity::model::application::CommandDataOption],
) {
    let label = opts
        .iter()
        .find(|o| o.name == "label")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("")
        .trim();

    if label.is_empty() {
        edit_response_text(ctx, command, "label requis.").await;
        return;
    }

    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            edit_response_text(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let mut templates = match load_templates(ctx, &guild_id).await {
        Ok(t) => t,
        Err(e) => {
            edit_response_text(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let before = templates.len();
    templates.retain(|t| !t.label.eq_ignore_ascii_case(label));
    let removed = before - templates.len();

    if removed == 0 {
        edit_response_text(
            ctx,
            command,
            &format!("Aucun template trouve avec le label `{label}`."),
        )
        .await;
        return;
    }

    if let Err(e) = save_templates(ctx, &guild_id, &templates).await {
        edit_response_text(ctx, command, &format!("Erreur : {e}")).await;
        return;
    }

    let embed = success_embed("\u{2705} Template(s) supprime(s)")
        .field("Label", label, true)
        .field("Supprimes", removed.to_string(), true)
        .field("Total restant", templates.len().to_string(), true);

    crate::shared::discord_helpers::edit_response_embed(ctx, command, embed).await;
}

async fn load_templates(ctx: &Context, guild_id: &str) -> Result<Vec<ReasonTemplate>, String> {
    let data = ctx.data.read().await;
    let base = data
        .get::<ApiClientKey>()
        .ok_or_else(|| "ApiClientKey manquant".to_string())?;
    let config = base
        .get_guild_config_for(guild_id, crate::modules::moderation::MODULE_BOT_NAME)
        .await
        .map_err(|e| format!("fetch config: {e}"))?;
    let raw = BaseApiClient::config_or(&config, CONFIG_KEY, "");
    Ok(reason_templates::parse_templates(&raw))
}

async fn save_templates(
    ctx: &Context,
    guild_id: &str,
    templates: &[ReasonTemplate],
) -> Result<(), String> {
    let serialized = reason_templates::serialize_templates(templates);

    let data = ctx.data.read().await;
    let api = data
        .get::<ModerationApiKey>()
        .ok_or_else(|| "ModerationApiKey manquant".to_string())?;
    api.set_bot_config(guild_id, BOT_NAME, CONFIG_KEY, &serialized)
        .await;
    Ok(())
}

// `serialize_templates` vit dans le core, à côté de `parse_templates`
// (l'aller-retour du format "label|raison" ne peut plus diverger).
