//! Module confessions : slash command /confess + panel persistant +
//! gestion modales (Submit, Reply, Report) + boutons + edit/delete par
//! admin via slash. API source de verite : tout passe par sentinel-api.

use std::sync::Arc;

use serenity::all::{
    ButtonStyle, CommandDataOptionValue, CommandInteraction, ComponentInteraction, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal,
    ModalInteraction,
};
use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateInputText, CreateMessage,
};
use serenity::model::application::{CommandOptionType, InputTextStyle};
use serenity::model::id::{ChannelId, GuildId, MessageId};
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

mod api_client;
use api_client::{ConfessionConfigData, ConfessionsApi};

pub const MODULE_BOT_NAME: &str = "confessions";

// ── Custom IDs ──────────────────────────────────────────────────────────

pub const CID_SUBMIT_BUTTON: &str = "conf_submit"; // bouton panel
pub const CID_REPLY_BUTTON_PREFIX: &str = "conf_reply:"; // conf_reply:<conf_id>
pub const CID_REPORT_BUTTON_PREFIX: &str = "conf_report:"; // conf_report:<conf_id>
pub const CID_SUBMIT_MODAL: &str = "conf_modal_submit";
pub const CID_REPLY_MODAL_PREFIX: &str = "conf_modal_reply:";
pub const CID_REPORT_MODAL_PREFIX: &str = "conf_modal_report:";

pub fn handles_component(cid: &str) -> bool {
    cid == CID_SUBMIT_BUTTON
        || cid.starts_with(CID_REPLY_BUTTON_PREFIX)
        || cid.starts_with(CID_REPORT_BUTTON_PREFIX)
}

pub fn handles_modal(cid: &str) -> bool {
    cid == CID_SUBMIT_MODAL
        || cid.starts_with(CID_REPLY_MODAL_PREFIX)
        || cid.starts_with(CID_REPORT_MODAL_PREFIX)
}

pub fn register_commands() -> Vec<CreateCommand> {
    vec![
        CreateCommand::new("confess")
            .description("Poste une confession anonyme dans le canal configure"),
        CreateCommand::new("confess-admin")
            .description("Administration des confessions (admin only)")
            .default_member_permissions(serenity::all::Permissions::MANAGE_GUILD)
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "deploy-panel",
                "Poste le bouton 'Poster une confession' dans ce canal",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "delete",
                    "Supprime une confession par numero",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "number",
                        "Numero de confession (ex: 350)",
                    )
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "reveal",
                    "Revele l'auteur d'une confession (gestion du serveur requise)",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "number",
                        "Numero de confession",
                    )
                    .required(true),
                ),
            ),
    ]
}

// ── Slash command dispatcher ────────────────────────────────────────────

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    let name = command.data.name.as_str();
    if name == "confess" {
        // Ouvre la modale de submission directement
        open_submit_modal(ctx, command).await;
        return;
    }
    if name != "confess-admin" {
        return;
    }
    // SECURITE : default_member_permissions n'est qu'un hint UI (override par
    // les params de guild ou une interaction forgee). On revalide MANAGE_GUILD
    // explicitement, sinon n'importe qui pourrait /confess-admin reveal et
    // de-anonymiser une confession (cf. revue securite).
    if !confess_admin_allowed(command) {
        reply_ephemeral(
            ctx,
            command,
            "Permission MANAGE_GUILD requise pour /confess-admin.",
        )
        .await;
        warn!(user = %command.user.name, user_id = %command.user.id, "Tentative /confess-admin sans permission");
        return;
    }
    // Sub-command
    let sub = command.data.options.first();
    let sub_name = sub.map(|o| o.name.as_str()).unwrap_or("");
    match sub_name {
        "deploy-panel" => admin_deploy_panel(ctx, command).await,
        "delete" => admin_delete(ctx, command).await,
        "reveal" => admin_reveal(ctx, command).await,
        _ => reply_ephemeral(ctx, command, "Sous-commande inconnue").await,
    }
}

/// Revalide la permission serveur pour /confess-admin (le flag
/// default_member_permissions est juste un hint UI Discord).
fn confess_admin_allowed(command: &CommandInteraction) -> bool {
    command
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| {
            p.contains(serenity::all::Permissions::MANAGE_GUILD)
                || p.contains(serenity::all::Permissions::ADMINISTRATOR)
        })
        .unwrap_or(false)
}

async fn open_submit_modal(ctx: &Context, command: &CommandInteraction) {
    let ui = load_ui_config(ctx, command.guild_id).await;
    let modal = CreateModal::new(CID_SUBMIT_MODAL, "Confession anonyme").components(vec![
        CreateActionRow::InputText(
            CreateInputText::new(InputTextStyle::Paragraph, "Ton message", "content")
                .min_length(ui.min_chars)
                .max_length(ui.max_chars)
                .required(true),
        ),
    ]);
    let resp = CreateInteractionResponse::Modal(modal);
    if let Err(e) = command.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec ouverture modale confess");
    }
}

/// Embed du panneau "Poster une confession" (partage deploy + repost collant).
fn panel_embed() -> CreateEmbed {
    CreateEmbed::new()
        .title("📝 Confessions anonymes")
        .description(
            "Clique sur le bouton ci-dessous pour poster une confession **anonyme**.\n\
             Personne (sauf le bot) ne saura qui a écrit. Sois respectueux et lis les règles.",
        )
        .color(0x5865f2)
}

/// Composants (bouton) du panneau.
fn panel_components() -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![CreateButton::new(
        CID_SUBMIT_BUTTON,
    )
    .label("Poster une confession")
    .style(ButtonStyle::Primary)
    .emoji('📝')])]
}

async fn admin_deploy_panel(ctx: &Context, command: &CommandInteraction) {
    let channel = command.channel_id;
    let msg = CreateMessage::new()
        .embed(panel_embed())
        .components(panel_components());
    match channel.send_message(&ctx.http, msg).await {
        Ok(message) => {
            let guild_id = command.guild_id.map(|g| g.to_string()).unwrap_or_default();
            // Persiste UNIQUEMENT le salon + l'id du panneau dans la source
            // unique (bot_guild_config, composant `confessions`). On n'ecrit
            // PLUS de reglages codes en dur (cooldown/max/min...) : deployer le
            // panneau ne doit jamais reinitialiser le tuning d'un serveur.
            if let Some(api) = api_client(ctx).await {
                persist_confession_setting(&api, &guild_id, "channel_id", &channel.to_string())
                    .await;
                persist_confession_setting(
                    &api,
                    &guild_id,
                    "panel_message_id",
                    &message.id.to_string(),
                )
                .await;
            }
            reply_ephemeral(ctx, command, "✅ Panel deploye dans ce canal.").await;
        }
        Err(e) => {
            warn!(error = %e, "Echec deploy panel");
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}

async fn admin_delete(ctx: &Context, command: &CommandInteraction) {
    let number = sub_int_option(command, "number").unwrap_or(0);
    if number <= 0 {
        reply_ephemeral(ctx, command, "Numero invalide").await;
        return;
    }
    let guild_id = command.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let api = match confessions_api(ctx).await {
        Some(a) => a,
        None => return,
    };
    // Trouve la confession par numero
    let list = match api.list(&guild_id, 500, false).await {
        Ok(l) => l,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };
    let target = match list.into_iter().find(|c| c.public_number == number) {
        Some(t) => t,
        None => {
            reply_ephemeral(ctx, command, &format!("Confession #{} introuvable", number)).await;
            return;
        }
    };
    let resp = api
        .delete(
            &target.id,
            &command.user.id.to_string(),
            Some("Supprimee par admin via slash command"),
        )
        .await;
    match resp {
        Ok(_) => {
            // Supprime aussi le message Discord (best-effort)
            if let (Some(ch), Some(msg)) =
                (target.channel_id.as_deref(), target.message_id.as_deref())
            {
                if let (Ok(c), Ok(m)) = (ch.parse::<u64>(), msg.parse::<u64>()) {
                    let _ = ChannelId::new(c)
                        .delete_message(&ctx.http, MessageId::new(m))
                        .await;
                }
            }
            reply_ephemeral(
                ctx,
                command,
                &format!("✅ Confession #{} supprimee", number),
            )
            .await;
        }
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}

async fn admin_reveal(ctx: &Context, command: &CommandInteraction) {
    let number = sub_int_option(command, "number").unwrap_or(0);
    if number <= 0 {
        reply_ephemeral(ctx, command, "Numero invalide").await;
        return;
    }
    let guild_id = command.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let api = match confessions_api(ctx).await {
        Some(a) => a,
        None => return,
    };
    let list = match api.list(&guild_id, 500, true).await {
        Ok(l) => l,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };
    match list.into_iter().find(|c| c.public_number == number) {
        Some(t) => {
            let author = if t.author_user_id.is_empty() {
                "?".to_string()
            } else {
                t.author_user_id
            };
            reply_ephemeral(
                ctx,
                command,
                &format!(
                    "Confession #{} → auteur : <@{}> (`{}`)",
                    number, author, author
                ),
            )
            .await;
        }
        None => {
            reply_ephemeral(ctx, command, &format!("Confession #{} introuvable", number)).await;
        }
    }
}

fn sub_int_option(command: &CommandInteraction, name: &str) -> Option<i64> {
    let sub = command.data.options.first()?;
    if let CommandDataOptionValue::SubCommand(opts) = &sub.value {
        for o in opts {
            if o.name == name {
                if let CommandDataOptionValue::Integer(v) = &o.value {
                    return Some(*v);
                }
            }
        }
    }
    None
}

// ── Component (boutons) ─────────────────────────────────────────────────

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.as_str();
    if cid == CID_SUBMIT_BUTTON {
        open_submit_modal_from_component(ctx, component).await;
        return;
    }
    if let Some(conf_id) = cid.strip_prefix(CID_REPLY_BUTTON_PREFIX) {
        open_reply_modal(ctx, component, conf_id).await;
        return;
    }
    if let Some(conf_id) = cid.strip_prefix(CID_REPORT_BUTTON_PREFIX) {
        open_report_modal(ctx, component, conf_id).await;
    }
}

async fn open_submit_modal_from_component(ctx: &Context, component: &ComponentInteraction) {
    let ui = load_ui_config(ctx, component.guild_id).await;
    let modal = CreateModal::new(CID_SUBMIT_MODAL, "Confession anonyme").components(vec![
        CreateActionRow::InputText(
            CreateInputText::new(InputTextStyle::Paragraph, "Ton message", "content")
                .min_length(ui.min_chars)
                .max_length(ui.max_chars)
                .required(true),
        ),
    ]);
    let resp = CreateInteractionResponse::Modal(modal);
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec ouverture modale submit");
    }
}

async fn open_reply_modal(ctx: &Context, component: &ComponentInteraction, conf_id: &str) {
    let ui = load_ui_config(ctx, component.guild_id).await;
    let modal = CreateModal::new(
        format!("{}{}", CID_REPLY_MODAL_PREFIX, conf_id),
        "Reponse anonyme",
    )
    .components(vec![CreateActionRow::InputText(
        CreateInputText::new(InputTextStyle::Paragraph, "Ta reponse", "content")
            .min_length(ui.min_chars)
            .max_length(ui.max_chars)
            .required(true),
    )]);
    let resp = CreateInteractionResponse::Modal(modal);
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec ouverture modale reply");
    }
}

async fn open_report_modal(ctx: &Context, component: &ComponentInteraction, conf_id: &str) {
    let ui = load_ui_config(ctx, component.guild_id).await;
    let modal = CreateModal::new(
        format!("{}{}", CID_REPORT_MODAL_PREFIX, conf_id),
        "Signaler cette confession",
    )
    .components(vec![CreateActionRow::InputText(
        CreateInputText::new(InputTextStyle::Paragraph, "Raison du signalement", "reason")
            .min_length(3)
            .max_length(ui.report_reason_max_len)
            .required(true),
    )]);
    let resp = CreateInteractionResponse::Modal(modal);
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec ouverture modale report");
    }
}

// ── Modal (submit / reply / report) ──────────────────────────────────────

pub async fn on_modal(ctx: &Context, modal: &ModalInteraction) {
    let cid = modal.data.custom_id.as_str();
    if cid == CID_SUBMIT_MODAL {
        handle_submit(ctx, modal).await;
        return;
    }
    if let Some(conf_id) = cid.strip_prefix(CID_REPLY_MODAL_PREFIX) {
        handle_reply(ctx, modal, conf_id).await;
        return;
    }
    if let Some(conf_id) = cid.strip_prefix(CID_REPORT_MODAL_PREFIX) {
        handle_report(ctx, modal, conf_id).await;
    }
}

async fn handle_submit(ctx: &Context, modal: &ModalInteraction) {
    // ACK immediat (deferred ephemere) : la suite enchaine plusieurs appels HTTP
    // (creation, config, embed, thread, panneau) qui depassent les 3s Discord.
    // Sans ce defer, l'interaction echoue et l'utilisateur resoumet -> confessions
    // dupliquees. Les reponses ulterieures passent en followup (cf. helper).
    let _ = modal
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await;
    let content = extract_input(modal, "content").unwrap_or_default();
    let guild_id = modal.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let user_id = modal.user.id.to_string();

    // base (HTTP) pour la config transverse `/api/bots/config` (repost_panel) ;
    // api (gRPC) pour les operations confessions.
    let (base, api) = match (api_client(ctx).await, confessions_api(ctx).await) {
        (Some(b), Some(a)) => (b, a),
        _ => return,
    };

    // 1. Cree la confession via API
    let created = match api.create(&guild_id, &user_id, &content).await {
        Ok(c) => c,
        Err(e) => {
            modal_reply_ephemeral(ctx, modal, &format!("❌ {}", e)).await;
            return;
        }
    };
    let id = created.id;
    let public_number = created.public_number;

    // 2. Recupere la config (gardee entiere pour pouvoir republier le panneau).
    let cfg = api.get_config(&guild_id).await.ok();
    let channel_id_str = cfg
        .as_ref()
        .and_then(|c| c.channel_id.clone())
        .unwrap_or_default();
    if channel_id_str.is_empty() {
        modal_reply_ephemeral(
            ctx,
            modal,
            "❌ Aucun salon de confession configure. Lance /confess-admin deploy-panel.",
        )
        .await;
        return;
    }
    let ch = match channel_id_str.parse::<u64>() {
        Ok(c) => ChannelId::new(c),
        Err(_) => return,
    };

    // 3. Poste l'embed sur Discord
    let ui = load_ui_config(ctx, modal.guild_id).await;
    let embed_color = ui.embed_color;
    let embed = CreateEmbed::new()
        .author(serenity::builder::CreateEmbedAuthor::new(format!(
            "Confession anonyme (#{})",
            public_number
        )))
        .description(&content)
        .color(embed_color);
    let row = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{}{}", CID_REPLY_BUTTON_PREFIX, id))
            .label("Répondre")
            .style(ButtonStyle::Secondary)
            .emoji('💬'),
        CreateButton::new(format!("{}{}", CID_REPORT_BUTTON_PREFIX, id))
            .label("Signaler")
            .style(ButtonStyle::Secondary)
            .emoji('🚩'),
    ]);
    let msg_payload = CreateMessage::new().embed(embed).components(vec![row]);
    let posted = ch.send_message(&ctx.http, msg_payload).await;
    let posted = match posted {
        Ok(m) => m,
        Err(e) => {
            modal_reply_ephemeral(ctx, modal, &format!("❌ Erreur post Discord : {e}")).await;
            return;
        }
    };

    // 4. Cree le thread "Confession Replies (#N)".
    // auto_archive_duration : le thread s'archive (= se ferme/repli) apres ce
    // delai d'inactivite. Discord n'autorise que 60min / 1j / 3j / 1 semaine.
    // 1h par defaut pour garder le salon propre ; un thread archive se rouvre
    // automatiquement des qu'une nouvelle reponse y est postee.
    let thread_name = format!("Confession Replies (#{})", public_number);
    let thread = ch
        .create_thread_from_message(
            &ctx.http,
            posted.id,
            serenity::builder::CreateThread::new(thread_name)
                .auto_archive_duration(ui.thread_archive_duration()),
        )
        .await
        .ok();
    let thread_id = thread.as_ref().map(|t| t.id.to_string());

    // 5. Update message_refs cote API
    let _ = api
        .update_message_refs(&id, &posted.id.to_string(), &ch.to_string(), thread_id)
        .await;

    // 6. "Message collant" : on republie le panneau EN BAS du salon pour que le
    // bouton "Poster une confession" reste accessible sans remonter le fil.
    if let Some(cfg) = cfg {
        repost_panel(ctx, &base, ch, cfg).await;
    }

    modal_reply_ephemeral(
        ctx,
        modal,
        &format!("✅ Confession #{} postee anonymement", public_number),
    )
    .await;
}

/// Republie le panneau en bas du salon (sticky) : poste un nouveau panneau,
/// met a jour `panel_message_id` dans la source unique (bot_guild_config), puis
/// supprime l'ancien panneau. On ne touche a AUCUN autre reglage.
async fn repost_panel(
    ctx: &Context,
    api: &Arc<BaseApiClient>,
    channel: ChannelId,
    cfg: ConfessionConfigData,
) {
    let old_panel_id = cfg.panel_message_id.clone();
    let guild_id = cfg.guild_id.clone();

    let posted = match channel
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .embed(panel_embed())
                .components(panel_components()),
        )
        .await
    {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "Echec repost panneau confession");
            return;
        }
    };

    // Persiste uniquement le nouveau panel_message_id (source unique).
    if !guild_id.is_empty() {
        persist_confession_setting(api, &guild_id, "panel_message_id", &posted.id.to_string())
            .await;
    }

    // Supprime l'ancien panneau (best-effort ; ignore si deja absent).
    if let Some(old) = old_panel_id {
        if old != posted.id.to_string() {
            if let Ok(mid) = old.parse::<u64>() {
                let _ = channel.delete_message(&ctx.http, MessageId::new(mid)).await;
            }
        }
    }
}

async fn handle_reply(ctx: &Context, modal: &ModalInteraction, conf_id: &str) {
    // ACK immediat (defer ephemere) : idem handle_submit, evite le timeout 3s
    // et les resoumissions.
    let _ = modal
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await;
    let content = extract_input(modal, "content").unwrap_or_default();
    let user_id = modal.user.id.to_string();
    let api = match confessions_api(ctx).await {
        Some(a) => a,
        None => return,
    };

    let created = match api.create_reply(conf_id, &user_id, &content, true).await {
        Ok(v) => v,
        Err(e) => {
            modal_reply_ephemeral(ctx, modal, &format!("❌ {}", e)).await;
            return;
        }
    };
    let reply_id = created.id;
    let public_number = created.public_number;

    // Recupere la confession pour avoir le thread_id
    let thread_id_str = api.get(conf_id).await.ok().and_then(|c| c.thread_id);
    let Some(thread_id) = thread_id_str else {
        modal_reply_ephemeral(ctx, modal, "❌ Thread introuvable").await;
        return;
    };
    let ch = match thread_id.parse::<u64>() {
        Ok(c) => ChannelId::new(c),
        Err(_) => return,
    };
    let embed_color = load_ui_config(ctx, modal.guild_id).await.embed_color;
    let embed = CreateEmbed::new()
        .author(serenity::builder::CreateEmbedAuthor::new(format!(
            "Réponse anonyme (#{})",
            public_number
        )))
        .description(&content)
        .color(embed_color);
    let posted = ch
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await;
    if let Ok(m) = posted {
        let _ = api
            .update_reply_message_id(&reply_id, &m.id.to_string())
            .await;
    }
    modal_reply_ephemeral(ctx, modal, "✅ Reponse anonyme postee").await;
}

async fn handle_report(ctx: &Context, modal: &ModalInteraction, conf_id: &str) {
    let reason = extract_input(modal, "reason").unwrap_or_default();
    let guild_id = modal.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let api = match confessions_api(ctx).await {
        Some(a) => a,
        None => return,
    };
    let resp = api
        .create_report(
            &guild_id,
            Some(conf_id),
            None,
            &modal.user.id.to_string(),
            &reason,
        )
        .await;
    match resp {
        Ok(_) => modal_reply_ephemeral(ctx, modal, "✅ Signalement transmis aux moderateurs").await,
        Err(e) => modal_reply_ephemeral(ctx, modal, &format!("❌ {}", e)).await,
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn extract_input(modal: &ModalInteraction, field_id: &str) -> Option<String> {
    for row in &modal.data.components {
        for c in &row.components {
            if let serenity::all::ActionRowComponent::InputText(it) = c {
                if it.custom_id == field_id {
                    return it.value.clone();
                }
            }
        }
    }
    None
}

async fn api_client(ctx: &Context) -> Option<Arc<BaseApiClient>> {
    let data = ctx.data.read().await;
    data.get::<ApiClientKey>().cloned()
}

/// Client gRPC du `ConfessionsService`, construit depuis le client partage.
async fn confessions_api(ctx: &Context) -> Option<ConfessionsApi> {
    let data = ctx.data.read().await;
    data.get::<crate::shared::grpc_client::GrpcClientKey>()
        .cloned()
        .map(ConfessionsApi::new)
}

/// Ecrit une cle de reglage confessions dans la source unique
/// (`bot_guild_config`, composant `confessions`) via l'endpoint generique
/// `/api/bots/config`. Best-effort : on log l'erreur sans bloquer.
async fn persist_confession_setting(
    api: &Arc<BaseApiClient>,
    guild_id: &str,
    key: &str,
    value: &str,
) {
    let body = serde_json::json!({
        "guild_id": guild_id,
        "bot_name": MODULE_BOT_NAME,
        "config_key": key,
        "config_value": value,
    });
    api.post_fire_and_forget("/api/bots/config", &body).await;
}

/// Reglages d'affichage des confessions lus depuis la config guild
/// (`bot_guild_config` du module `confessions`, editables via le dashboard).
struct ConfessUiConfig {
    /// min_length du champ de la modale (>= 1, <= max_chars).
    min_chars: u16,
    /// max_length du champ de la modale (<= 4000, borne modale Discord).
    max_chars: u16,
    /// Couleur de l'embed des confessions/reponses.
    embed_color: u32,
    /// Duree d'auto-archivage du thread de reponses (minutes). Discord n'autorise
    /// que 60 / 1440 / 4320 / 10080. Defaut 60.
    thread_archive_minutes: u16,
    /// max_length du champ "raison" de la modale de signalement (<= 4000). Defaut 500.
    report_reason_max_len: u16,
}

impl Default for ConfessUiConfig {
    fn default() -> Self {
        Self {
            min_chars: 5,
            max_chars: 2000,
            embed_color: 0xff5e5e,
            thread_archive_minutes: 60,
            report_reason_max_len: 500,
        }
    }
}

impl ConfessUiConfig {
    /// Traduit `thread_archive_minutes` en `AutoArchiveDuration` Discord.
    /// Toute valeur hors des paliers autorises retombe sur 1h.
    fn thread_archive_duration(&self) -> serenity::all::AutoArchiveDuration {
        use serenity::all::AutoArchiveDuration::*;
        match self.thread_archive_minutes {
            1440 => OneDay,
            4320 => ThreeDays,
            10080 => OneWeek,
            _ => OneHour,
        }
    }
}

/// Parse une couleur hex ("#ff5e5e" ou "ff5e5e") en u32 RGB. `None` si invalide.
use platform_core::sentinel::domain::services::system::discord_naming::parse_hex_color_strict as parse_hex_color;

/// Charge les reglages d'affichage depuis la config guild `confessions` via le
/// meme mecanisme `get_guild_config_for` que les autres modules. Fallback sur
/// les defauts (min 5 / max 2000 / #ff5e5e) si config absente ou invalide.
/// Les bornes de longueur sont clampees au max modal Discord (4000) et
/// `min_chars` est borne a `max_chars`.
async fn load_ui_config(ctx: &Context, guild_id: Option<GuildId>) -> ConfessUiConfig {
    let mut cfg = ConfessUiConfig::default();
    let (Some(api), Some(gid)) = (api_client(ctx).await, guild_id) else {
        return cfg;
    };
    let Ok(entries) = api
        .get_guild_config_for(&gid.to_string(), MODULE_BOT_NAME)
        .await
    else {
        return cfg;
    };
    let max = BaseApiClient::config_u64(&entries, "max_chars", 2000).clamp(1, 4000);
    let min = BaseApiClient::config_u64(&entries, "min_chars", 5).clamp(1, max);
    cfg.max_chars = max as u16;
    cfg.min_chars = min as u16;
    if let Some(c) = entries
        .get("default_embed_color_hex")
        .and_then(|v| parse_hex_color(v))
    {
        cfg.embed_color = c;
    }
    // Duree d'archivage du thread (paliers Discord). Valeur inconnue -> defaut 60.
    let archive = BaseApiClient::config_u64(&entries, "thread_archive_minutes", 60);
    cfg.thread_archive_minutes = match archive {
        1440 => 1440,
        4320 => 4320,
        10080 => 10080,
        _ => 60,
    };
    cfg.report_reason_max_len =
        BaseApiClient::config_u64(&entries, "report_reason_max_len", 500).clamp(1, 4000) as u16;
    cfg
}

async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    );
    if let Err(e) = command.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec reply ephemere confess");
    }
}

async fn modal_reply_ephemeral(ctx: &Context, modal: &ModalInteraction, content: &str) {
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    );
    // Si l'interaction a deja ete acquittee (Defer en amont pour ACK &lt; 3s), le
    // create_response echoue -> on retombe sur un followup ephemere.
    if modal.create_response(&ctx.http, resp).await.is_err() {
        let _ = modal
            .create_followup(
                &ctx.http,
                serenity::builder::CreateInteractionResponseFollowup::new()
                    .content(content)
                    .ephemeral(true),
            )
            .await;
    }
}

// ── Consumer Redis stream pour sync bidirectionnelle Web -> Discord ─────

/// Spawn le consumer durable Redis stream sentinel:events filtre sur les
/// events "confession_deleted" et "confession_reply_deleted". Quand un
/// admin supprime une confession via la page web, l'API broadcast un event
/// avec message_id+channel_id, et ce consumer supprime le message Discord
/// pour garder la sync.
pub fn spawn_consumer(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "sentinel-bot-confessions".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move { handle_event(&ctx, &payload_json).await }
            },
        )
        .await;
    });
}

async fn handle_event(ctx: &Context, payload_json: &str) {
    let envelope: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    let event = envelope.get("event").and_then(|v| v.as_str()).unwrap_or("");
    let data = match envelope.get("data") {
        Some(d) => d.clone(),
        None => return,
    };
    match event {
        "confession_deleted" => {
            let channel_id_str = data
                .get("channel_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let message_id_str = data
                .get("message_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if channel_id_str.is_empty() || message_id_str.is_empty() {
                return;
            }
            let (Ok(c), Ok(m)) = (channel_id_str.parse::<u64>(), message_id_str.parse::<u64>())
            else {
                return;
            };
            let ch = ChannelId::new(c);
            let mid = MessageId::new(m);
            // Idempotent : si deja supprime, on ignore l'erreur 404.
            match ch.delete_message(&ctx.http, mid).await {
                Ok(_) => info!(
                    channel_id = c,
                    message_id = m,
                    "Confession message deleted (sync from web)"
                ),
                Err(e) => {
                    let s = e.to_string();
                    if !s.contains("404") {
                        warn!(error = %e, "Echec delete message confession (sync web)");
                    }
                }
            }
        }
        "confession_reply_deleted" => {
            // Le reply est dans le thread - on doit retrouver le channel.
            // L'API broadcast n'envoie pas le channel_id du thread, donc on
            // recupere via la confession parent.
            let confession_id = data
                .get("confession_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let message_id_str = data
                .get("message_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if confession_id.is_empty() || message_id_str.is_empty() {
                return;
            }
            let api = match confessions_api(ctx).await {
                Some(a) => a,
                None => return,
            };
            let thread_id_str = match api.get(confession_id).await {
                Ok(c) => c.thread_id.unwrap_or_default(),
                Err(_) => return,
            };
            if thread_id_str.is_empty() {
                return;
            }
            let (Ok(c), Ok(m)) = (thread_id_str.parse::<u64>(), message_id_str.parse::<u64>())
            else {
                return;
            };
            let ch = ChannelId::new(c);
            let mid = MessageId::new(m);
            match ch.delete_message(&ctx.http, mid).await {
                Ok(_) => info!(
                    thread_id = c,
                    message_id = m,
                    "Reply message deleted (sync web)"
                ),
                Err(e) => {
                    let s = e.to_string();
                    if !s.contains("404") {
                        warn!(error = %e, "Echec delete reply message (sync web)");
                    }
                }
            }
        }
        _ => {}
    }
}
