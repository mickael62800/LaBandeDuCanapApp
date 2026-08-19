//! Module idees — boite a idees du serveur, calquee sur les tickets.
//!
//! Flow : panneau public -> bouton « Proposer une idee » -> choix de la
//! categorie -> modale (titre + description) -> salon prive auteur + staff,
//! avec une carte portant les boutons de decision reserves au staff.
//! Les messages du salon sont synchronises vers l'API pour etre relus depuis
//! le web, ou le staff peut aussi trancher (event Redis `idea_decided`).

pub const MODULE_BOT_NAME: &str = "idea-bot";

pub mod api_client;
pub mod constants;
pub mod embed;
pub mod events;
pub mod interactions;
pub mod panel;

use serenity::all::{
    CommandInteraction, CommandOptionType, ComponentInteraction, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
    ModalInteraction,
};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use tracing::{info, warn};

use crate::shared::discord_helpers::{
    is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
    is_module_enabled_or_reply_modal,
};

use constants::*;

// ── Slash commands ──

pub fn register_commands() -> Vec<CreateCommand> {
    vec![CreateCommand::new("idee")
        .description("Boite a idees du serveur")
        .default_member_permissions(serenity::all::Permissions::MANAGE_GUILD)
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "panneau",
            "Poste le panneau de proposition d'idees dans ce salon",
        ))]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if command.data.name != "idee" {
        return;
    }
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }

    // Poster le panneau est une action de configuration : reservee a « Gerer le
    // serveur », sinon n'importe qui pourrait le dupliquer partout.
    let allowed = command
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.manage_guild() || p.administrator())
        .unwrap_or(false);
    if !allowed {
        respond(
            ctx,
            command,
            "Reserve au staff (permission Gerer le serveur).",
        )
        .await;
        return;
    }

    match command
        .channel_id
        .send_message(&ctx.http, panel::build_panel_message())
        .await
    {
        Ok(_) => respond(ctx, command, "Panneau des idees poste.").await,
        Err(e) => {
            warn!(error = %e, "Echec envoi du panneau des idees");
            respond(ctx, command, "Echec de l'envoi du panneau.").await;
        }
    }
}

async fn respond(ctx: &Context, command: &CommandInteraction, content: &str) {
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    );
    if let Err(e) = command.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec reponse commande /idee");
    }
}

// ── Component interactions ──

pub fn handles_component(cid: &str) -> bool {
    cid == PANEL_BUTTON_ID || cid == CATEGORY_SELECT_ID || status_for_button(cid).is_some()
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }
    let cid = component.data.custom_id.as_str();
    if cid == PANEL_BUTTON_ID {
        panel::handle_panel_click(ctx, component).await;
    } else if cid == CATEGORY_SELECT_ID {
        panel::handle_category_select(ctx, component).await;
    } else if status_for_button(cid).is_some() {
        interactions::handle_status_button(ctx, component).await;
    }
}

// ── Modal interactions ──

pub fn handles_modal(cid: &str) -> bool {
    cid.starts_with(MODAL_ID_PREFIX) || cid.starts_with(REASON_MODAL_PREFIX)
}

pub async fn on_modal(ctx: &Context, modal: &ModalInteraction) {
    if !is_module_enabled_or_reply_modal(ctx, modal, MODULE_BOT_NAME).await {
        return;
    }
    let cid = modal.data.custom_id.as_str();
    // L'ordre compte : les deux prefixes commencent par "idea_".
    if cid.starts_with(REASON_MODAL_PREFIX) {
        interactions::handle_reason_modal(ctx, modal).await;
    } else if cid.starts_with(MODAL_ID_PREFIX) {
        panel::handle_modal_submit(ctx, modal).await;
    }
}

// ── Ready / background ──

pub async fn on_ready(_ctx: &Context, _ready: &Ready) {
    info!("Module idees pret");
}

/// Consumer des decisions prises depuis le web.
pub fn spawn_background(ctx: Context) {
    events::spawn(ctx);
}

// ── Sync des messages du salon d'une idee ──

/// Recopie les messages des salons d'idees vers l'API pour qu'ils soient
/// relisibles depuis le web. Best-effort : silencieux si le salon n'est pas
/// un salon d'idee.
pub async fn on_message(ctx: &Context, msg: &Message) {
    if msg.author.bot || msg.content.trim().is_empty() {
        return;
    }
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return,
    };
    // Filtre bon marche avant tout appel reseau : seuls les salons "idee-*"
    // sont candidats.
    let is_idea_channel = msg
        .channel_id
        .name(&ctx.http)
        .await
        .map(|n| n.starts_with("idee-"))
        .unwrap_or(false);
    if !is_idea_channel {
        return;
    }
    if !crate::shared::discord_helpers::is_module_enabled(
        ctx,
        &guild_id.to_string(),
        MODULE_BOT_NAME,
    )
    .await
    {
        return;
    }

    let grpc = {
        let data = ctx.data.read().await;
        match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
            Some(g) => g.clone(),
            None => return,
        }
    };
    let api = api_client::ApiClient::new(grpc);
    let idea = match api.idea_by_channel(&msg.channel_id.to_string()).await {
        Ok(i) => i,
        Err(_) => return,
    };
    let role = if msg.author.id.to_string() == idea.author_id {
        "auteur"
    } else {
        "staff"
    };
    api.add_message(&idea.id, &msg.author.name, role, &msg.content)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commande_idee_est_reservee_aux_gestionnaires_du_serveur() {
        let commands = register_commands();
        let json = serde_json::to_value(&commands[0]).expect("commande serialisable");

        assert_eq!(
            json.get("default_member_permissions")
                .and_then(|v| v.as_str()),
            Some("32")
        );
    }
}
