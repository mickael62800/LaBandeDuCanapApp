//! Boutons de decision du staff, et modale de motif associee.

use std::collections::HashMap;

use serenity::all::{
    ComponentInteraction, Context, CreateActionRow, CreateInputText, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateModal, EditInteractionResponse, InputTextStyle,
    ModalInteraction,
};
use serenity::builder::CreateMessage;
use tracing::{info, warn};

use crate::modules::ideas::api_client::{ApiClient, Idea};
use crate::modules::ideas::embed::{build_idea_embed_full, build_staff_buttons};
use crate::modules::ideas::MODULE_BOT_NAME;
use crate::shared::heartbeat::ApiClientKey;

use super::constants::*;

/// Le membre porte-t-il le role staff configure ?
///
/// Fail-closed : sans role configure ou sans membre resolu, seule la permission
/// Discord « Gerer le serveur » ouvre les boutons.
fn is_staff_member(member: Option<&serenity::all::Member>, cfg: &HashMap<String, String>) -> bool {
    let member = match member {
        Some(m) => m,
        None => return false,
    };
    if let Some(perms) = member.permissions {
        if perms.manage_guild() || perms.administrator() {
            return true;
        }
    }
    let staff_role = match cfg.get("staff_role_id").and_then(|v| v.parse::<u64>().ok()) {
        Some(r) => serenity::model::id::RoleId::new(r),
        None => return false,
    };
    member.roles.contains(&staff_role)
}

/// Clic sur un bouton de decision -> ouvre la modale de motif.
pub async fn handle_status_button(ctx: &Context, component: &ComponentInteraction) {
    let status = match status_for_button(&component.data.custom_id) {
        Some(s) => s,
        None => return,
    };
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };

    let cfg = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(base) => base
                .get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME)
                .await
                .unwrap_or_default(),
            None => HashMap::new(),
        }
    };

    if !is_staff_member(component.member.as_ref(), &cfg) {
        let resp = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Seul le staff peut statuer sur une idee.")
                .ephemeral(true),
        );
        if let Err(e) = component.create_response(&ctx.http, resp).await {
            warn!(error = %e, "Echec reponse refus staff");
        }
        return;
    }

    let reason_input = CreateInputText::new(
        InputTextStyle::Paragraph,
        "Motif (visible par l'auteur)",
        FIELD_REASON,
    )
    .placeholder("Explique la decision : pourquoi, quand, sous quelle forme...")
    .required(false)
    .max_length(1000);

    let modal = CreateModal::new(
        format!("{REASON_MODAL_PREFIX}{status}"),
        format!("Idee — {}", status_label(status)),
    )
    .components(vec![CreateActionRow::InputText(reason_input)]);

    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
        .await
    {
        warn!(error = %e, "Echec ouverture modale de motif");
    }
}

/// Soumission du motif -> applique la decision et met a jour le salon.
pub async fn handle_reason_modal(ctx: &Context, modal: &ModalInteraction) {
    let status = match modal.data.custom_id.strip_prefix(REASON_MODAL_PREFIX) {
        Some(s) => s.to_string(),
        None => return,
    };
    let guild_id = match modal.guild_id {
        Some(g) => g,
        None => return,
    };

    let mut reason = String::new();
    for row in &modal.data.components {
        for comp in &row.components {
            if let serenity::all::ActionRowComponent::InputText(input) = comp {
                if input.custom_id == FIELD_REASON {
                    reason = input.value.clone().unwrap_or_default();
                }
            }
        }
    }

    if let Err(e) = modal
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Echec defer modale de motif");
    }

    // `base` sert uniquement a lire la config guild (toujours en HTTP) ;
    // les operations sur les idees passent par gRPC.
    let (grpc, cfg) = {
        let data = ctx.data.read().await;
        let base = match data.get::<ApiClientKey>() {
            Some(b) => b.clone(),
            None => return,
        };
        let grpc = match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
            Some(g) => g.clone(),
            None => return,
        };
        let cfg = base
            .get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME)
            .await
            .unwrap_or_default();
        (grpc, cfg)
    };

    // La permission est revalidee a la soumission : un utilisateur dont le
    // role staff a ete retire apres l'ouverture de la modale ne peut pas
    // encore appliquer une decision administrative.
    if !is_staff_member(modal.member.as_ref(), &cfg) {
        edit(ctx, modal, "Seul le staff peut statuer sur une idee.").await;
        warn!(
            user = %modal.user.name,
            user_id = %modal.user.id,
            "Tentative de decision d'idee sans permission staff"
        );
        return;
    }
    let api = ApiClient::new(grpc);

    // L'idee est retrouvee par le salon d'ou vient le clic.
    let idea = match api.idea_by_channel(&modal.channel_id.to_string()).await {
        Ok(i) => i,
        Err(e) => {
            warn!(error = %e, "Aucune idee rattachee a ce salon");
            edit(ctx, modal, "Ce salon n'est rattache a aucune idee.").await;
            return;
        }
    };

    let reason_opt = Some(reason.as_str()).filter(|r| !r.trim().is_empty());
    let updated = match api
        .decide(
            &idea.id,
            &status,
            &modal.user.id.to_string(),
            &modal.user.name,
            reason_opt,
        )
        .await
    {
        Ok(i) => i,
        Err(e) => {
            // Couvre notamment les transitions interdites (ex. realisee sans
            // acceptation prealable), refusees par le domaine.
            warn!(error = %e, idea = %idea.id, "Decision refusee");
            edit(ctx, modal, &format!("Decision impossible : {e}")).await;
            return;
        }
    };

    announce_decision(ctx, modal, &updated, &cfg).await;

    info!(
        idea = %updated.id,
        status = %updated.status,
        by = %modal.user.name,
        "Statut d'idee mis a jour"
    );
    edit(
        ctx,
        modal,
        &format!("Idee marquee « {} ».", status_label(&updated.status)),
    )
    .await;
}

/// Poste la carte mise a jour dans le salon de l'idee et previent l'auteur.
async fn announce_decision(
    ctx: &Context,
    modal: &ModalInteraction,
    idea: &Idea,
    cfg: &HashMap<String, String>,
) {
    let decided_by = idea
        .decided_by_name
        .clone()
        .unwrap_or_else(|| "Staff".into());
    let embed = build_idea_embed_full(
        &idea.id,
        &idea.title,
        &idea.description,
        &idea.category,
        &idea.status,
        &idea.author_name,
        None,
        Some((decided_by.as_str(), idea.decision_reason.as_deref())),
        cfg,
    );

    // Les boutons restent tant que l'idee n'est pas terminale : le staff peut
    // encore revenir sur sa decision.
    let mut message = CreateMessage::new().embed(embed);
    if idea.status != "realisee" {
        message = message.components(vec![build_staff_buttons()]);
    }
    if let Err(e) = modal.channel_id.send_message(&ctx.http, message).await {
        warn!(error = %e, "Echec publication de la decision dans le salon");
    }

    // DM a l'auteur : il n'a pas forcement le salon ouvert.
    if let Ok(uid) = idea.author_id.parse::<u64>() {
        let user_id = serenity::model::id::UserId::new(uid);
        let motif = idea
            .decision_reason
            .as_deref()
            .filter(|r| !r.trim().is_empty())
            .map(|r| format!("\nMotif : {r}"))
            .unwrap_or_default();
        let text = format!(
            "Ton idee « {} » est passee au statut **{}**.{motif}",
            idea.title,
            status_label(&idea.status)
        );
        if let Ok(channel) = user_id.create_dm_channel(&ctx.http).await {
            if let Err(e) = channel.say(&ctx.http, text).await {
                tracing::debug!(error = %e, "DM de decision non delivre (DM fermes ?)");
            }
        }
    }
}

async fn edit(ctx: &Context, modal: &ModalInteraction, content: &str) {
    if let Err(e) = modal
        .edit_response(&ctx.http, EditInteractionResponse::new().content(content))
        .await
    {
        warn!(error = %e, "Echec reponse modale de motif");
    }
}
