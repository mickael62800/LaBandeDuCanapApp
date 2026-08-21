//! Panneau permanent de la Roue du Destin.
//!
//! Porte depuis `sentinel-bot/src/modules/wheel/{setup,buttons}.rs`, supprime
//! au commit ff6e8a46 quand les jeux ont quitte sentinel. Le portage vers
//! nexus n'avait repris que `/roue` ; le panneau manquait depuis.
//!
//! Pourquoi un panneau plutot que la seule commande : `/roue` suppose de
//! connaitre la commande. Un bouton epingle en bas du salon se voit, et c'est
//! ce qui fait revenir les gens tous les jours.
//!
//! Deux differences assumees avec l'original :
//!
//!   - la duree de l'animation est une constante. Dans sentinel elle etait
//!     lue dans la config du serveur ; nexus n'a pas encore de config par
//!     guilde cote bot, et un reglage qu'aucun service ne lit vaut moins que
//!     pas de reglage du tout.
//!   - pas de journal de jeu : nexus n'a pas de salon de logs configurable.

use std::time::Duration;

use serenity::all::{
    ButtonStyle, ChannelId, CommandInteraction, ComponentInteraction, Context, CreateActionRow,
    CreateButton, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, EditMessage, GetMessages, MessageId, Permissions,
};
use tracing::{info, warn};

use crate::api_client::ApiClient;
use crate::embeds;

/// `custom_id` du bouton. Stable et sans donnee variable : le panneau doit
/// rester cliquable des semaines apres avoir ete poste, y compris apres un
/// redemarrage du bot.
pub const PANEL_SPIN_ID: &str = "roue_panel_spin";

/// Titre EXACT du panneau. Sert a reperer les anciens panneaux a supprimer
/// lors du repost collant : il ne doit donc jamais etre le titre d'un autre
/// embed du bot, sous peine de voir les resultats de tirage disparaitre.
pub const PANEL_TITLE: &str = "\u{1f300} La Roue du Destin";

/// Duree du suspense entre l'annonce du tirage et son resultat.
const SPIN_ANIMATION_MS: u64 = 4000;

pub fn register() -> CreateCommand {
    CreateCommand::new("roue-panel")
        .description("Deployer le panneau de la Roue du Destin dans ce salon (admin)")
        .default_member_permissions(Permissions::MANAGE_GUILD)
}

pub fn handles_component(custom_id: &str) -> bool {
    custom_id == PANEL_SPIN_ID
}

/// Le message du panneau : embed + bouton. Partage entre le deploiement
/// initial et le repost collant, pour que les deux ne puissent pas diverger.
pub fn build_panel_message() -> CreateMessage {
    let embed = serenity::all::CreateEmbed::new()
        .title(PANEL_TITLE)
        .description(
            "\u{1fa99} **Une fois par jour**, tente ta chance.\n\n\
             Le destin peut te rendre **riche** (jackpot, licorne) ou **ridicule**\n\
             (PQ, ruine, bombe). Un seul tirage par jour, alors choisis bien... ou pas.\n\n\
             *Le resultat est annonce publiquement. Tout le salon en parle.*",
        )
        .color(0xf1c40f);

    let button = CreateButton::new(PANEL_SPIN_ID)
        .label("Tirer la Roue")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{1f300}".into(),
        ))
        .style(ButtonStyle::Success);

    CreateMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::Buttons(vec![button])])
}

/// « Message collant » : republie le panneau EN BAS du salon et supprime les
/// anciens. Sans ca, chaque resultat de tirage repousse le panneau vers le
/// haut et il devient introuvable au bout de trois messages.
///
/// Best-effort de bout en bout : un scan ou une suppression qui echoue est
/// journalise puis ignore. Le pire cas est un panneau en double, pas un salon
/// sans panneau.
pub async fn repost_panel(ctx: &Context, channel_id: ChannelId) {
    let bot_id = ctx.cache.current_user().id;

    // Reperer les anciens AVANT de poster le nouveau : dans l'autre ordre on
    // supprimerait celui qu'on vient d'envoyer.
    let old_panels: Vec<MessageId> = match channel_id
        .messages(&ctx.http, GetMessages::new().limit(50))
        .await
    {
        Ok(messages) => messages
            .into_iter()
            .filter(|m| {
                m.author.id == bot_id
                    && m.embeds
                        .iter()
                        .any(|e| e.title.as_deref() == Some(PANEL_TITLE))
            })
            .map(|m| m.id)
            .collect(),
        Err(e) => {
            warn!(error = %e, "scan des anciens panneaux Roue impossible");
            Vec::new()
        }
    };

    if let Err(e) = channel_id
        .send_message(&ctx.http, build_panel_message())
        .await
    {
        warn!(error = %e, "repost du panneau Roue impossible");
        return;
    }

    for id in old_panels {
        if let Err(e) = channel_id.delete_message(&ctx.http, id).await {
            warn!(error = %e, message_id = %id, "suppression d'un ancien panneau Roue impossible");
        }
    }
}

pub fn is_panel_admin(permissions: Option<Permissions>) -> bool {
    permissions.is_some_and(|p| {
        p.contains(Permissions::MANAGE_GUILD) || p.contains(Permissions::ADMINISTRATOR)
    })
}

pub const MSG_PERMISSION_REQUIRED: &str = "Permission « Gerer le serveur » requise.";
pub const MSG_DEPLOY_FAILED: &str = "Deploiement du panneau impossible.";
pub const MSG_DEPLOY_SUCCESS: &str = "Panneau de la Roue deploye !";

/// `/roue-panel` — pose le panneau dans le salon courant.
pub async fn handle_command(ctx: &Context, cmd: &CommandInteraction) {
    // Fail-closed : sans permission LISIBLE dans l'interaction, on refuse.
    // Discord filtre deja la commande via `default_member_permissions`, mais
    // ce filtre est cote client et se contourne.
    let autorise = is_panel_admin(cmd.member.as_ref().and_then(|m| m.permissions));
    if !autorise {
        reply_ephemeral(ctx, cmd, MSG_PERMISSION_REQUIRED).await;
        return;
    }

    if let Err(e) = cmd
        .channel_id
        .send_message(&ctx.http, build_panel_message())
        .await
    {
        warn!(error = %e, "envoi du panneau Roue impossible");
        reply_ephemeral(ctx, cmd, MSG_DEPLOY_FAILED).await;
        return;
    }

    reply_ephemeral(ctx, cmd, MSG_DEPLOY_SUCCESS).await;
    info!(channel = %cmd.channel_id, "panneau Roue deploye");
}

/// Clic sur « Tirer la Roue ».
///
/// L'ordre compte : on appelle l'API AVANT d'annoncer quoi que ce soit. Poster
/// « la roue tourne » puis se prendre un refus laisserait un message mensonger
/// dans le salon. Le refus, lui, reste prive : personne n'a besoin de savoir
/// que quelqu'un a reclique.
pub async fn handle_spin(api: &ApiClient, ctx: &Context, component: &ComponentInteraction) {
    let Some(guild_id) = component.guild_id else {
        return;
    };
    let username = component.user.display_name().to_string();

    if let Err(e) = component
        .create_response(&ctx.http, build_spin_defer_response())
        .await
    {
        warn!(error = %e, "defer du tirage impossible");
        return;
    }

    let response = match api
        .spin_wheel(
            &guild_id.to_string(),
            &component.user.id.to_string(),
            &username,
        )
        .await
    {
        Ok(r) => r,
        Err(message) => {
            let edit = build_spin_error_edit(&message);
            let _ = component.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    // Le suspense est public : c'est ce qui fait lever les tetes dans le
    // salon. Le resultat viendra remplacer ce message.
    let mut annonce = match component
        .channel_id
        .send_message(&ctx.http, build_spin_announce_message(&username))
        .await
    {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "annonce du tirage impossible");
            return;
        }
    };

    tokio::time::sleep(Duration::from_millis(SPIN_ANIMATION_MS)).await;

    if let Err(e) = annonce
        .edit(&ctx.http, build_spin_result_edit(&response, &username))
        .await
    {
        warn!(error = %e, "affichage du resultat impossible");
    }

    // Le resultat vient de repousser le panneau vers le haut : on le remet en
    // bas.
    repost_panel(ctx, component.channel_id).await;

    let edit = build_spin_final_edit(&response.case_label);
    let _ = component.edit_response(&ctx.http, edit).await;
}

pub fn build_spin_defer_response() -> CreateInteractionResponse {
    CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true),
    )
}

pub fn build_spin_announce_message(username: &str) -> CreateMessage {
    CreateMessage::new().embed(embeds::build_spinning_embed(username))
}

pub fn build_spin_result_edit(
    response: &crate::api_client::WheelSpinResponse,
    username: &str,
) -> EditMessage {
    EditMessage::new().embed(embeds::build_result_embed(response, username))
}

pub fn build_spin_error_edit(message: &str) -> serenity::builder::EditInteractionResponse {
    serenity::builder::EditInteractionResponse::new()
        .embed(embeds::build_error_embed(message))
}

pub fn build_spin_final_edit(case_label: &str) -> serenity::builder::EditInteractionResponse {
    serenity::builder::EditInteractionResponse::new()
        .content(format_spin_result_response(case_label))
}

pub fn is_old_wheel_panel(author_id: serenity::all::UserId, bot_id: serenity::all::UserId, titles: &[Option<&str>]) -> bool {
    author_id == bot_id && titles.iter().any(|t| *t == Some(PANEL_TITLE))
}

pub fn filter_old_wheel_panel_ids(
    messages: &[(MessageId, serenity::all::UserId, Vec<Option<String>>)],
    bot_id: serenity::all::UserId,
) -> Vec<MessageId> {
    messages
        .iter()
        .filter_map(|(id, author, titles)| {
            if *author == bot_id && titles.iter().any(|t| t.as_deref() == Some(PANEL_TITLE)) {
                Some(*id)
            } else {
                None
            }
        })
        .collect()
}

pub fn format_spin_result_response(case_label: &str) -> String {
    format!("\u{1f300} Ton tirage : {}", case_label)
}

pub fn build_ephemeral_reply(message: &str) -> CreateInteractionResponse {
    CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(message)
            .ephemeral(true),
    )
}

async fn reply_ephemeral(ctx: &Context, cmd: &CommandInteraction, message: &str) {
    let _ = cmd.create_response(&ctx.http, build_ephemeral_reply(message)).await;
}

pub async fn execute_spin(
    api: &ApiClient,
    guild_id: &str,
    user_id: &str,
    username: &str,
) -> Result<crate::api_client::WheelSpinResponse, String> {
    api.spin_wheel(guild_id, user_id, username).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serenity::all::UserId;

    #[test]
    fn test_register() {
        let cmd = register();
        let json = serde_json::to_value(&cmd).unwrap();
        assert_eq!(json["name"], "roue-panel");
    }

    #[test]
    fn test_handles_component() {
        assert!(handles_component(PANEL_SPIN_ID));
        assert!(!handles_component("other_button"));
    }

    #[test]
    fn test_build_panel_message() {
        let msg = build_panel_message();
        let json = serde_json::to_value(&msg).unwrap();
        assert!(json["components"].as_array().is_some());
    }

    #[test]
    fn test_is_panel_admin() {
        assert!(!is_panel_admin(None));
        assert!(!is_panel_admin(Some(Permissions::SEND_MESSAGES)));
        assert!(is_panel_admin(Some(Permissions::MANAGE_GUILD)));
        assert!(is_panel_admin(Some(Permissions::ADMINISTRATOR)));
    }

    #[test]
    fn test_wheel_panel_helpers() {
        let bot = UserId::new(100);
        let other = UserId::new(200);

        assert!(is_old_wheel_panel(bot, bot, &[Some(PANEL_TITLE)]));
        assert!(!is_old_wheel_panel(other, bot, &[Some(PANEL_TITLE)]));
        assert!(!is_old_wheel_panel(bot, bot, &[Some("Autre titre")]));

        let msgs = vec![
            (MessageId::new(1), bot, vec![Some(PANEL_TITLE.to_string())]),
            (MessageId::new(2), other, vec![Some(PANEL_TITLE.to_string())]),
            (MessageId::new(3), bot, vec![Some("Autre".to_string())]),
            (MessageId::new(4), bot, vec![None]),
        ];
        let old_ids = filter_old_wheel_panel_ids(&msgs, bot);
        assert_eq!(old_ids, vec![MessageId::new(1)]);

        let res = format_spin_result_response("Jackpot 1000");
        assert!(res.contains("Jackpot 1000"));

        let err_edit = build_spin_error_edit("API down");
        let j_err = serde_json::to_value(&err_edit).unwrap();
        assert!(j_err["embeds"].as_array().is_some());

        let final_edit = build_spin_final_edit("PQ 100");
        let j_final = serde_json::to_value(&final_edit).unwrap();
        assert!(j_final["content"].as_str().unwrap().contains("PQ 100"));

        let eph = build_ephemeral_reply("Succès");
        let j_eph = serde_json::to_value(&eph).unwrap();
        assert_eq!(j_eph["data"]["content"], "Succès");

        let def = build_spin_defer_response();
        let j_def = serde_json::to_value(&def).unwrap();
        assert_eq!(j_def["type"], 5); // Defer

        let ann = build_spin_announce_message("Alice");
        let j_ann = serde_json::to_value(&ann).unwrap();
        assert!(j_ann["embeds"].as_array().is_some());

        let res_obj = crate::api_client::WheelSpinResponse {
            case_label: "Jackpot".into(),
            payout: 1000,
            balance_after: 2000,
            is_memorable: true,
        };
        let res_edit = build_spin_result_edit(&res_obj, "Alice");
        let j_res_edit = serde_json::to_value(&res_edit).unwrap();
        assert!(j_res_edit["embeds"].as_array().is_some());
    }

    #[tokio::test]
    async fn test_execute_spin() {
        use tokio::net::TcpListener;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf).await.unwrap_or(0);

                let body = r#"{"case_label":"Licorne 🦄","payout":500,"balance_after":1500,"is_memorable":true}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            }
        });

        let client = ApiClient::new(base_url, Some("token".into()));
        let res = execute_spin(&client, "g1", "u1", "Alice").await;
        assert!(res.is_ok());
        let spin = res.unwrap();
        assert_eq!(spin.payout, 500);
        assert_eq!(spin.balance_after, 1500);
        assert!(spin.is_memorable);

        assert!(MSG_PERMISSION_REQUIRED.contains("Gerer le serveur"));
        assert!(MSG_DEPLOY_FAILED.contains("impossible"));
        assert!(MSG_DEPLOY_SUCCESS.contains("deploye"));
    }

    #[test]
    fn test_format_spin_result_response_various() {
        let res1 = format_spin_result_response("Lucky 7");
        assert!(res1.contains("Lucky 7"));
        assert!(res1.contains("🌀"));

        let res2 = format_spin_result_response("Gold Bar 1000");
        assert!(res2.contains("Gold Bar 1000"));

        let res3 = format_spin_result_response("");
        assert!(res3.contains("🌀"));

        let res4 = format_spin_result_response("Çøsé ☕");
        assert!(res4.contains("Çøsé ☕"));
    }

    #[test]
    fn test_build_ephemeral_reply_content() {
        let reply1 = build_ephemeral_reply("Success!");
        let j1 = serde_json::to_value(&reply1).unwrap();
        assert_eq!(j1["data"]["content"], "Success!");

        let reply2 = build_ephemeral_reply("");
        let j2 = serde_json::to_value(&reply2).unwrap();
        assert_eq!(j2["data"]["content"], "");

        let reply3 = build_ephemeral_reply("Very long message with lots of text to test edge cases");
        let j3 = serde_json::to_value(&reply3).unwrap();
        assert!(j3["data"]["content"].as_str().unwrap().len() > 0);
    }

    #[test]
    fn test_is_old_wheel_panel_edge_cases() {
        let bot = UserId::new(100);
        let other = UserId::new(200);

        // Bot with correct title
        assert!(is_old_wheel_panel(bot, bot, &[Some(PANEL_TITLE)]));

        // Bot with wrong author
        assert!(!is_old_wheel_panel(other, bot, &[Some(PANEL_TITLE)]));

        // Bot with wrong title
        assert!(!is_old_wheel_panel(bot, bot, &[Some("Wrong")]));

        // Empty titles
        assert!(!is_old_wheel_panel(bot, bot, &[]));

        // Title in middle
        assert!(is_old_wheel_panel(bot, bot, &[Some("Other"), Some(PANEL_TITLE), Some("More")]));

        // None titles
        assert!(!is_old_wheel_panel(bot, bot, &[None, None]));
    }

    #[test]
    fn test_filter_old_wheel_panel_ids_comprehensive() {
        let bot = UserId::new(100);
        let other = UserId::new(200);

        let msgs = vec![
            (MessageId::new(1), bot, vec![Some(PANEL_TITLE.to_string())]),
            (MessageId::new(2), bot, vec![Some("Wrong".to_string())]),
            (MessageId::new(3), other, vec![Some(PANEL_TITLE.to_string())]),
            (MessageId::new(4), bot, vec![]),
            (MessageId::new(5), bot, vec![None]),
            (MessageId::new(6), bot, vec![Some("X".to_string()), Some(PANEL_TITLE.to_string())]),
        ];

        let filtered = filter_old_wheel_panel_ids(&msgs, bot);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&MessageId::new(1)));
        assert!(filtered.contains(&MessageId::new(6)));
    }

    #[test]
    fn test_filter_old_wheel_panel_empty() {
        let bot = UserId::new(100);
        let msgs = vec![];
        let filtered = filter_old_wheel_panel_ids(&msgs, bot);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_old_wheel_panel_no_matches() {
        let bot = UserId::new(100);
        let other = UserId::new(200);

        let msgs = vec![
            (MessageId::new(1), other, vec![Some(PANEL_TITLE.to_string())]),
            (MessageId::new(2), bot, vec![Some("Wrong".to_string())]),
        ];

        let filtered = filter_old_wheel_panel_ids(&msgs, bot);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_build_spin_defer_response() {
        let defer = build_spin_defer_response();
        let json = serde_json::to_value(&defer).unwrap();
        assert_eq!(json["type"], 5); // DeferredChannelMessage
    }

    #[test]
    fn test_build_spin_error_edit() {
        let edit = build_spin_error_edit("Error message");
        let json = serde_json::to_value(&edit).unwrap();
        assert!(json["embeds"].as_array().is_some());
    }

    #[test]
    fn test_build_spin_final_edit() {
        let edit = build_spin_final_edit("PQ 100");
        let json = serde_json::to_value(&edit).unwrap();
        let content = json["content"].as_str().unwrap();
        assert!(content.contains("PQ 100"));
        assert!(content.contains("🌀"));
    }
}
