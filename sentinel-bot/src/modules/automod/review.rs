//! Review mode : cartes de validation moderateur + handlers des boutons.

use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::embeds::danger_embed;
use crate::shared::heartbeat::ApiClientKey;

use super::api_client::Action;
use super::config::{build_embed_colors, EmbedColors};
use super::detectors;
use super::{AM_PREFIX, DEFAULT_MUTE_DURATION_SECS};

/// Mappe un type d'action automod (warn/mute/ban) vers la `SanctionKind` de la
/// card de sanction. `None` pour les actions sans card (delete/prevention/ignore).
/// Partage par la review 1-clic et la finalisation de vote (BUG #4).
pub(crate) fn sanction_kind_for(
    action_type: &str,
) -> Option<crate::shared::discord_helpers::SanctionKind> {
    use crate::shared::discord_helpers::SanctionKind;
    match action_type {
        "warn" => Some(SanctionKind::Warn),
        "mute" => Some(SanctionKind::Mute),
        "ban" => Some(SanctionKind::Ban),
        _ => None,
    }
}

/// Envoie une carte de review dans le salon de logs au lieu d'appliquer
/// l'action directement. Les moderateurs cliquent sur un bouton pour
/// valider ou ajuster la severite.
#[allow(clippy::too_many_arguments)]
pub(super) async fn send_review_card(
    ctx: &Context,
    msg: &Message,
    suggested_action: &Action,
    reason: &str,
    score: f64,
    flags: &detectors::DetectionFlags,
    review_channel_id: u64,
    colors: &EmbedColors,
    // Note d'action automatique deja appliquee (mute/suppression) a afficher
    // sur la carte. `None` = aucune action auto.
    auto_note: Option<String>,
    // `true` si l'auto-protection a DÉJÀ journalisé une sanction de membre pour
    // cet incident -> la finalisation de la carte ne la re-journalisera pas
    // (anti double-strike, cf. C1).
    already_sanctioned: bool,
) {
    let guild_id = msg.guild_id.map(|g| g.to_string()).unwrap_or_default();

    // Mode VOTE : si vote_enabled, on delegue a la carte de vote des
    // moderateurs (au lieu de la validation 1-clic). On capture aussi
    // `discussion_enabled` pour le bouton « Ouvrir une discussion » du 1-clic.
    let mut discussion_enabled = false;
    let mut detail_url: Option<String> = None;
    {
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            let cfg = api
                .get_guild_config_for(&guild_id, super::MODULE_BOT_NAME)
                .await
                .unwrap_or_default();
            discussion_enabled =
                BaseApiClient::config_bool(&cfg, "discussion_channel_enabled", false);
            detail_url = super::vote::build_detail_url(&cfg, &guild_id);
            // Modération humaine : human_only force le mode vote (1 carte/personne
            // agregee + decision humaine), au lieu des cartes 1-clic non agregees.
            let force_vote = BaseApiClient::config_bool(&cfg, "human_only_enabled", false);
            if force_vote || BaseApiClient::config_bool(&cfg, "vote_enabled", false) {
                let deadline_hours =
                    BaseApiClient::config_u64(&cfg, "vote_deadline_hours", 72) as i64;
                let context_before =
                    BaseApiClient::config_u64(&cfg, "vote_context_before", 10) as u8;
                let thread_enabled = BaseApiClient::config_bool(&cfg, "vote_thread_enabled", true);
                let aggregate = BaseApiClient::config_bool(&cfg, "vote_aggregate_enabled", false);
                let aggregate_window =
                    BaseApiClient::config_u64(&cfg, "vote_aggregate_window_minutes", 60) as i64;
                drop(data);
                super::vote::post_vote_card(
                    ctx,
                    msg,
                    suggested_action,
                    reason,
                    score,
                    flags,
                    review_channel_id,
                    deadline_hours,
                    context_before,
                    thread_enabled,
                    aggregate,
                    aggregate_window,
                    discussion_enabled,
                    detail_url,
                    auto_note,
                    already_sanctioned,
                )
                .await;
                return;
            }
        }
    }

    let channel_id = msg.channel_id.to_string();
    let message_id = msg.id.to_string();
    let user_id = msg.author.id.to_string();
    let content_preview = sanitize_embed_content(&msg.content, 500);

    let action_label = match suggested_action {
        Action::Warn => "Avertissement",
        Action::Delete => "Suppression",
        Action::Mute => "Mute",
        Action::Kick => "Kick",
        Action::Ban => "Bannissement",
        Action::None => return,
    };

    let action_color = match suggested_action {
        Action::Warn => colors.warn,
        Action::Delete => colors.delete,
        Action::Mute => colors.mute,
        Action::Kick => colors.ban,
        Action::Ban => colors.ban,
        Action::None => 0x95a5a6,
    };

    let mut flag_parts = Vec::new();
    if flags.spam {
        flag_parts.push("Spam");
    }
    if flags.insult {
        flag_parts.push("Insulte");
    }
    if flags.link {
        flag_parts.push("Lien");
    }
    if flags.phishing {
        flag_parts.push("Phishing");
    }
    let flags_str = if flag_parts.is_empty() {
        "Aucun".to_string()
    } else {
        flag_parts.join(", ")
    };

    let mut embed = serenity::builder::CreateEmbed::new()
        .title(format!("AutoMod -- Action suggeree : {}", action_label))
        .color(action_color)
        .field(
            "Utilisateur",
            format!("<@{}> (`{}`)", user_id, msg.author.name),
            true,
        )
        .field("Salon", format!("<#{}>", channel_id), true)
        .field("Score IA", format!("{:.2}", score), true)
        .field(
            "Message original",
            format!("```{}```", content_preview),
            false,
        )
        .field("Raison IA", reason, false)
        .field("Flags detectes", &flags_str, true)
        .thumbnail(msg.author.face())
        .footer(serenity::builder::CreateEmbedFooter::new(
            "AutoMod Review | Cliquez pour valider ou ajuster",
        ))
        .timestamp(serenity::model::Timestamp::now());
    // Action automatique deja appliquee (raid / phishing / pub / gros flood).
    if let Some(note) = &auto_note {
        embed = embed.field("🚨 Action automatique appliquee", note, false);
    }
    // 2e section : antecedents de moderation du membre (avec dates).
    if let Some(hist) = super::vote::render_history_totals(ctx, &guild_id, &user_id).await {
        embed = embed.field("📋 Antecedents du membre", hist, false);
    }

    // Suffixe commun pour les custom_id
    let id_suffix = format!("{}:{}:{}:{}", guild_id, channel_id, message_id, user_id);

    // Bouton principal (action suggeree) + ajustements + ignorer.
    let suggested_char = action_char(suggested_action);

    let btn_apply =
        serenity::builder::CreateButton::new(format!("am_{}:{}", suggested_char, id_suffix))
            .label(format!("Appliquer ({})", action_label))
            .style(serenity::all::ButtonStyle::Success);

    let btn_ignore = serenity::builder::CreateButton::new(format!("am_i:{}", id_suffix))
        .label("Ignorer")
        .style(serenity::all::ButtonStyle::Secondary);

    // Rangee 2 : ajustements de severite (sans doublon avec le bouton principal)
    let mut adjust_buttons = Vec::new();
    if suggested_char != 'w' {
        adjust_buttons.push(
            serenity::builder::CreateButton::new(format!("am_w:{}", id_suffix))
                .label("Warn")
                .style(serenity::all::ButtonStyle::Secondary),
        );
    }
    if suggested_char != 'd' {
        adjust_buttons.push(
            serenity::builder::CreateButton::new(format!("am_d:{}", id_suffix))
                .label("Delete")
                .style(serenity::all::ButtonStyle::Secondary),
        );
    }
    if suggested_char != 'm' {
        adjust_buttons.push(
            serenity::builder::CreateButton::new(format!("am_m:{}", id_suffix))
                .label("Mute")
                .style(serenity::all::ButtonStyle::Danger),
        );
    }

    // On cree la review en DB AVANT de poster la carte : on obtient son `id`,
    // ce qui permet d'ajouter le bouton « Ouvrir une discussion » (amdisc:<id>)
    // — meme bouton/handler que les cartes de vote (full hexa).
    let review_id = create_review_in_api(
        ctx,
        &guild_id,
        &channel_id,
        &message_id,
        &user_id,
        &msg.author.name,
        &content_preview,
        suggested_action,
        score,
        reason,
        flags,
        already_sanctioned,
    )
    .await;

    let row1 = serenity::builder::CreateActionRow::Buttons(vec![btn_apply, btn_ignore]);
    let row2 = serenity::builder::CreateActionRow::Buttons(adjust_buttons);
    let mut rows = vec![row1, row2];
    // 3e rangee : lien "Voir le detail" (dashboard) + (option) "Ouvrir une discussion".
    let mut extra: Vec<serenity::builder::CreateButton> = Vec::new();
    if let Some(url) = &detail_url {
        extra.push(serenity::builder::CreateButton::new_link(url).label("📋 Voir le détail"));
    }
    if discussion_enabled {
        if let Some(id) = &review_id {
            extra.push(
                serenity::builder::CreateButton::new(format!(
                    "{}{}",
                    super::vote::DISCUSSION_PREFIX,
                    id
                ))
                .label("Ouvrir une discussion")
                .style(serenity::all::ButtonStyle::Secondary),
            );
        }
    }
    if !extra.is_empty() {
        rows.push(serenity::builder::CreateActionRow::Buttons(extra));
    }

    let builder = serenity::builder::CreateMessage::new()
        .embed(embed)
        .components(rows);

    match serenity::model::id::ChannelId::new(review_channel_id)
        .send_message(&ctx.http, builder)
        .await
    {
        Ok(posted) => {
            info!(
                user = %msg.author.name,
                channel = %msg.channel_id,
                action = %action_label,
                review_channel = review_channel_id,
                "Carte de review envoyee"
            );
            // Mapping carte <-> review pour la sync web (edit bilateral).
            if let Some(id) = &review_id {
                register_review_mapping(
                    ctx,
                    id,
                    &guild_id,
                    &posted.channel_id.to_string(),
                    &posted.id.to_string(),
                )
                .await;
            }
        }
        Err(e) => error!(
            error = %e,
            review_channel = review_channel_id,
            "Echec envoi carte de review automod -- verifier que le bot a acces au salon"
        ),
    }
}

/// Cree la review en DB via l'API et retourne son `id` (UUID) pour pouvoir
/// construire les boutons qui en dependent (ex. discussion) puis enregistrer
/// le mapping `(action_id, carte Discord)`. `None` si creation impossible
/// (API down) — la carte Discord reste fonctionnelle (fire-and-forget).
async fn create_review_in_api(
    ctx: &Context,
    guild_id: &str,
    channel_id: &str,
    message_id: &str,
    user_id: &str,
    user_name: &str,
    content_preview: &str,
    suggested_action: &Action,
    score: f64,
    reason: &str,
    flags: &detectors::DetectionFlags,
    already_sanctioned: bool,
) -> Option<String> {
    let suggested_str = match suggested_action {
        Action::Warn => "warn",
        Action::Delete => "delete",
        Action::Mute => "mute",
        Action::Kick => "kick",
        Action::Ban => "ban",
        Action::None => return None,
    };
    let grpc = {
        let data = ctx.data.read().await;
        match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
            Some(g) => g.clone(),
            None => return None,
        }
    };
    let review_api = super::api_client::ApiClient::new(grpc);

    // Carte 1-clic (hors mode vote) : pas de `voting_deadline`, pas d'agregation.
    let resp = review_api
        .create_review(super::api_client::CreateReviewParams {
            guild_id,
            channel_id,
            message_id,
            user_id,
            user_name,
            content_preview,
            suggested_action: suggested_str,
            score,
            reason,
            flags: serde_json::json!({
                "spam": flags.spam,
                "insult": flags.insult,
                "link": flags.link,
                "phishing": flags.phishing,
            }),
            voting_deadline: None,
            aggregate: false,
            aggregate_window_minutes: None,
            already_sanctioned,
        })
        .await;
    match resp {
        Ok(r) => Some(r.id),
        Err(e) => {
            warn!(error = %e, "Echec creation automod review en DB (sync degrade)");
            None
        }
    }
}

/// Enregistre le mapping `(review_id, carte Discord)` dans
/// `discord_action_messages` pour que le web retrouve la carte lors d'une
/// resolution. Appele apres l'envoi de la carte (on a alors le message poste).
async fn register_review_mapping(
    ctx: &Context,
    review_id: &str,
    guild_id: &str,
    card_channel_id: &str,
    card_message_id: &str,
) {
    let Ok(uuid) = uuid::Uuid::parse_str(review_id) else {
        return;
    };
    let grpc = {
        let data = ctx.data.read().await;
        match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
            Some(g) => g.clone(),
            None => return,
        }
    };
    crate::sync::register_action_message(
        &grpc,
        uuid,
        crate::sync::kinds::AUTOMOD_REVIEW,
        guild_id,
        card_channel_id,
        card_message_id,
    )
    .await;
}

fn action_char(action: &Action) -> char {
    match action {
        Action::Warn => 'w',
        Action::Delete => 'd',
        Action::Mute => 'm',
        Action::Kick => 'k',
        Action::Ban => 'b',
        Action::None => 'i',
    }
}

fn char_to_action(c: char) -> Action {
    match c {
        'w' => Action::Warn,
        'd' => Action::Delete,
        'm' => Action::Mute,
        'k' => Action::Kick,
        'b' => Action::Ban,
        _ => Action::None,
    }
}

/// Handler des boutons de review. Parse le custom_id, execute l'action
/// choisie par le moderateur, et met a jour la carte.
pub(super) async fn handle_review_button(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    let has_perm = component
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| {
            p.contains(serenity::all::Permissions::MODERATE_MEMBERS)
                || p.contains(serenity::all::Permissions::MANAGE_MESSAGES)
                || p.contains(serenity::all::Permissions::ADMINISTRATOR)
        })
        .unwrap_or(false);

    if !has_perm {
        let _ = component
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .content("Seul un moderateur peut valider une action automod.")
                        .ephemeral(true),
                ),
            )
            .await;
        warn!(
            user = %component.user.name,
            user_id = %component.user.id,
            "Tentative d'action review sans permission"
        );
        return;
    }

    // Idempotence : la carte 1-clic applique la sanction SANS gate DB (le
    // bouton ne porte pas le review_id). On verrouille donc au niveau carte :
    // une seule action par carte, anti double-clic / double-modo simultane.
    let guard_key = format!("card:{}", component.message.id);
    if !super::claim_once(ctx, &guard_key).await {
        let _ = component
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .content("Cette carte a deja ete traitee.")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let custom_id = &component.data.custom_id;
    // Format : am_{action}:{guild_id}:{channel_id}:{message_id}:{user_id}
    let parts: Vec<&str> = custom_id.split(':').collect();
    if parts.len() != 5 {
        warn!(custom_id = %custom_id, "custom_id review malforme (nombre de parts incorrect)");
        return;
    }

    let action_str = match parts[0].strip_prefix(AM_PREFIX) {
        Some(s) => s,
        None => {
            warn!(custom_id = %custom_id, "custom_id review sans prefix am_");
            return;
        }
    };
    let action_char_val = match action_str.chars().next() {
        Some(c) if matches!(c, 'w' | 'd' | 'm' | 'b' | 'i') => c,
        _ => {
            warn!(custom_id = %custom_id, "custom_id review action char invalide");
            return;
        }
    };
    let action = char_to_action(action_char_val);
    let _guild_id_str = parts[1];
    let channel_id_str = parts[2];
    let message_id_str = parts[3];
    let user_id_str = parts[4];

    let moderator_name = &component.user.name;

    // Charger la config guild pour mute_duration
    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let data = ctx.data.read().await;
    let config = if let Some(api) = data.get::<ApiClientKey>() {
        api.get_guild_config_for(&guild_id, crate::modules::automod::MODULE_BOT_NAME)
            .await
            .unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };
    drop(data);

    let mute_duration_secs =
        BaseApiClient::config_u64(&config, "mute_duration_secs", DEFAULT_MUTE_DURATION_SECS);
    let colors = build_embed_colors(&config);
    let appeal = BaseApiClient::config_bool(&config, "sanction_appeal_enabled", true);
    let notify_member = BaseApiClient::config_bool(&config, "sanction_notify_member", true);

    if action == Action::None {
        // Ignorer -- mettre a jour la carte
        info!(target = %user_id_str, moderator = %moderator_name, "Detection ignoree via review");
        let ignored_embed = serenity::builder::CreateEmbed::new()
            .title("AutoMod -- Ignore par un moderateur")
            .description(format!(
                "Moderateur : **{}**\nAucune action appliquee.",
                moderator_name
            ))
            .color(0x95a5a6)
            .timestamp(serenity::model::Timestamp::now());

        if let Err(e) = component
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::UpdateMessage(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .embed(ignored_embed)
                        .components(vec![]),
                ),
            )
            .await
        {
            error!(error = %e, "Echec update carte review (ignore)");
        }
        return;
    }

    // Executer l'action sur le message original
    let action_label = match &action {
        Action::Warn => "Avertissement",
        Action::Delete => "Suppression",
        Action::Mute => "Mute",
        Action::Kick => "Kick",
        Action::Ban => "Bannissement",
        Action::None => "Aucune",
    };

    let channel_id = match channel_id_str.parse::<u64>() {
        Ok(id) => serenity::model::id::ChannelId::new(id),
        Err(_) => return,
    };

    // Execute l'action
    match action {
        Action::Warn => {
            info!(target = %user_id_str, channel = %channel_id_str, moderator = %moderator_name, "Warn valide via review");
            let embed = crate::shared::embeds::sanction_notice(
                "warn",
                "Contenu inapproprie detecte par l'IA",
                None,
                Some(moderator_name),
                appeal,
            );
            if let Err(e) = channel_id
                .send_message(
                    &ctx.http,
                    serenity::builder::CreateMessage::new().embed(embed),
                )
                .await
            {
                error!(error = %e, "Echec envoi embed warn dans le salon");
            }
        }
        Action::Delete => {
            if let Ok(msg_id) = message_id_str.parse::<u64>() {
                match channel_id
                    .delete_message(&ctx.http, serenity::model::id::MessageId::new(msg_id))
                    .await
                {
                    Ok(_) => info!(message_id = %msg_id, "Message supprime via review"),
                    Err(e) => {
                        warn!(error = %e, message_id = %msg_id, "Echec suppression message (peut-etre deja supprime)")
                    }
                }
            }
            let embed = crate::shared::embeds::sanction_notice(
                "delete",
                "Contenu inapproprie",
                None,
                Some(moderator_name),
                appeal,
            );
            if let Err(e) = channel_id
                .send_message(
                    &ctx.http,
                    serenity::builder::CreateMessage::new().embed(embed),
                )
                .await
            {
                error!(error = %e, "Echec envoi embed delete dans le salon");
            }
        }
        Action::Mute => {
            let mut mute_applied = false;
            if let (Some(guild_id_val), Ok(uid)) = (component.guild_id, user_id_str.parse::<u64>())
            {
                match crate::modules::moderation::role_mute::apply(
                    ctx,
                    guild_id_val,
                    serenity::model::id::UserId::new(uid),
                    mute_duration_secs,
                )
                .await
                {
                    Ok(crate::modules::moderation::role_mute::ApplyResult::Applied) => {
                        info!(user_id = %uid, duration = mute_duration_secs, "Mute applique via role depuis review");
                        mute_applied = true;
                    }
                    Ok(crate::modules::moderation::role_mute::ApplyResult::AlreadyActive) => {
                        info!(user_id = %uid, "Mute deja actif via role, aucune prolongation");
                    }
                    Err(e) => error!(error = %e, user_id = %uid, "Echec role de mute via review"),
                    Ok(crate::modules::moderation::role_mute::ApplyResult::NotConfigured) => {
                        match guild_id_val
                            .member(&ctx.http, serenity::model::id::UserId::new(uid))
                            .await
                        {
                            Ok(mut member) => {
                                let secs = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs() as i64 + mute_duration_secs as i64)
                                    .unwrap_or(0);
                                match time::OffsetDateTime::from_unix_timestamp(secs) {
                                    Ok(dt) => {
                                        let timeout = serenity::model::Timestamp::from(dt);
                                        match member
                                            .disable_communication_until_datetime(
                                                &ctx.http, timeout,
                                            )
                                            .await
                                        {
                                            Ok(_) => {
                                                info!(user_id = %uid, duration = mute_duration_secs, "Mute applique via review");
                                                mute_applied = true;
                                            }
                                            Err(e) => {
                                                error!(error = %e, user_id = %uid, "Echec Discord disable_communication -- le bot a-t-il la permission MODERATE_MEMBERS ?")
                                            }
                                        }
                                    }
                                    Err(e) => error!(error = %e, "Timestamp invalide pour mute"),
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, user_id = %uid, "Membre introuvable pour mute")
                            }
                        }
                    }
                }
            } else {
                warn!(guild_id = ?component.guild_id, user_id = %user_id_str, "guild_id ou user_id invalide pour mute");
            }

            // Supprimer le message original APRES le mute (best-effort).
            if let Ok(msg_id) = message_id_str.parse::<u64>() {
                if let Err(e) = channel_id
                    .delete_message(&ctx.http, serenity::model::id::MessageId::new(msg_id))
                    .await
                {
                    warn!(error = %e, "Echec suppression message apres mute review");
                }
            }
            let mute_min = mute_duration_secs / 60;
            let embed = if mute_applied {
                crate::shared::embeds::sanction_notice(
                    "mute",
                    "Contenu inapproprie",
                    Some(mute_min),
                    Some(moderator_name),
                    appeal,
                )
            } else {
                // Echec du mute : on reste factuel (pas un gabarit de sanction).
                danger_embed("Mute ECHOUE (voir logs)")
                    .color(colors.mute)
                    .field("Duree", format!("{} minutes", mute_min), true)
                    .field("Valide par", moderator_name.as_str(), true)
            };
            let _ = channel_id
                .send_message(
                    &ctx.http,
                    serenity::builder::CreateMessage::new().embed(embed),
                )
                .await;
        }
        Action::Kick => {
            if let (Some(guild_id_val), Ok(uid)) = (component.guild_id, user_id_str.parse::<u64>())
            {
                if let Err(e) = guild_id_val
                    .kick_with_reason(
                        &ctx.http,
                        serenity::model::id::UserId::new(uid),
                        "Sanction validee par un moderateur (AutoMod review)",
                    )
                    .await
                {
                    error!(error = %e, user_id = %uid, "Echec kick via review");
                }
            }
        }
        Action::Ban => {
            // Decision humaine -> ban reel (coherent avec la finalisation de vote).
            let _ = super::vote::apply_member_sanction(
                ctx,
                component.guild_id,
                channel_id_str,
                message_id_str,
                user_id_str,
                "ban",
                mute_duration_secs,
            )
            .await;
            info!(target = %user_id_str, channel = %channel_id_str, moderator = %moderator_name, "Ban applique via review");
            let embed = crate::shared::embeds::sanction_notice(
                "ban",
                "Contenu inapproprie",
                None,
                Some(moderator_name),
                appeal,
            );
            if let Err(e) = channel_id
                .send_message(
                    &ctx.http,
                    serenity::builder::CreateMessage::new().embed(embed),
                )
                .await
            {
                error!(error = %e, "Echec envoi embed ban dans le salon");
            }
        }
        Action::None => {}
    }

    if notify_member && action != Action::None {
        super::backend::send_sanction_dm(
            ctx,
            component.user.id,
            &action,
            "Sanction validée par un modérateur",
            mute_duration_secs,
            appeal,
            None,
        )
        .await;
    }

    // Trace la sanction de membre (warn/mute/ban) dans le module moderation
    // (historique + escalade), au meme titre que le vote et les commandes.
    let action_type = match action {
        Action::Warn => "warn",
        Action::Mute => "mute",
        Action::Kick => "kick",
        Action::Ban => "ban",
        _ => "",
    };
    if !action_type.is_empty() {
        super::vote::log_sanction_to_moderation(
            ctx,
            &guild_id,
            &component.channel_id.to_string(),
            &component.user.id.to_string(),
            &component.user.name,
            user_id_str,
            user_id_str,
            action_type,
            "Sanction validee par un moderateur (AutoMod review)",
            if action == Action::Mute {
                Some(mute_duration_secs)
            } else {
                None
            },
        )
        .await;

        // BUG #4 : card de sanction pour la review 1-clic (warn/mute/ban), au meme
        // titre que les sanctions manuelles et l'auto-mute automod. Best-effort.
        if let (Some(kind), Ok(uid)) = (sanction_kind_for(action_type), user_id_str.parse::<u64>())
        {
            let duration_label = if action == Action::Mute {
                Some(format!("{}min", mute_duration_secs / 60))
            } else {
                None
            };
            crate::shared::discord_helpers::post_sanction_card(
                ctx,
                &guild_id,
                kind,
                uid,
                None,
                moderator_name,
                "Sanction validee par un moderateur (AutoMod review)",
                duration_label.as_deref(),
            )
            .await;
        }
    }

    // 1-clic : si un salon de discussion a ete ouvert pour cette infraction, on
    // l'archive (snapshot transcript + suppression) maintenant que l'affaire est
    // close. Les boutons 1-clic ne portent pas le review_id -> on le retrouve par
    // le message d'infraction. No-op si aucune discussion.
    {
        let grpc = {
            let data = ctx.data.read().await;
            data.get::<crate::shared::grpc_client::GrpcClientKey>()
                .cloned()
        };
        if let Some(grpc) = grpc {
            let review_api = super::api_client::ApiClient::new(grpc);
            if let Ok(Some(r)) = review_api
                .find_review_by_message(&guild_id, message_id_str)
                .await
            {
                super::vote::archive_discussion_channel(ctx, &review_api, &r.id, &r.user_id).await;
            }
        }
    }

    // Mettre a jour la carte de review (retirer les boutons, afficher le resultat)
    let result_embed = serenity::builder::CreateEmbed::new()
        .title(format!("AutoMod -- {} applique", action_label))
        .description(format!(
            "Moderateur : **{}**\nCible : <@{}>\nSalon : <#{}>",
            moderator_name, user_id_str, channel_id_str
        ))
        .color(match action {
            Action::Warn => colors.warn,
            Action::Delete => colors.delete,
            Action::Mute => colors.mute,
            Action::Kick => colors.ban,
            Action::Ban => colors.ban,
            Action::None => 0x95a5a6,
        })
        .footer(serenity::builder::CreateEmbedFooter::new(
            "AutoMod Review | Action executee",
        ))
        .timestamp(serenity::model::Timestamp::now());

    let _ = component
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::UpdateMessage(
                serenity::builder::CreateInteractionResponseMessage::new()
                    .embed(result_embed)
                    .components(vec![]),
            ),
        )
        .await;

    info!(
        moderator = %moderator_name,
        action = %action_label,
        target_user = %user_id_str,
        "Action automod validee par un moderateur"
    );
}

/// Handler Redis Stream : `automod_review_resolved` depuis web.
/// Edite la carte Discord (gris + footer) et applique l'action en miroir
/// (warn/mute/ban/delete). Skip si `actor.source != "web"` (anti-boucle).
pub(super) async fn handle_redis_event(ctx: &Context, payload: &str) {
    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return,
    };

    let event_type = event.get("event").and_then(|e| e.as_str()).unwrap_or("");
    if event_type != "automod_review_resolved" {
        return;
    }
    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };
    let source = data
        .get("actor")
        .and_then(|a| a.get("source"))
        .and_then(|s| s.as_str())
        .unwrap_or("");
    if source != "web" {
        return;
    }
    let action_id = match data.get("action_id").and_then(|v| v.as_str()) {
        Some(a) if !a.is_empty() => a,
        _ => return,
    };
    let applied_action = data
        .get("applied_action")
        .and_then(|v| v.as_str())
        .unwrap_or("ignore");
    let actor_name = data
        .get("actor")
        .and_then(|a| a.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("Web admin");

    edit_review_card_from_web(ctx, action_id, applied_action, actor_name).await;

    // Applique l'action sur Discord (le handler web ne faisait qu'editer la
    // carte). La sanction est tracee cote API dans la requete /resolve.
    apply_web_resolution(ctx, action_id, applied_action).await;

    // L'affaire est close : on archive le salon de discussion lie (snapshot du
    // transcript en DB + verrouillage), comme la finalisation/cloture Discord.
    // Vaut pour TOUTE resolution web, y compris "ignore".
    {
        let grpc = {
            let data = ctx.data.read().await;
            match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
                Some(g) => g.clone(),
                None => return,
            }
        };
        let review_api = super::api_client::ApiClient::new(grpc);
        if let Ok(r) = review_api.get_review(action_id).await {
            super::vote::archive_discussion_channel(ctx, &review_api, action_id, &r.user_id).await;
        }
    }
}

/// Applique sur Discord la sanction resolue depuis le web (delete/mute/ban).
/// La tracabilite (historique moderation) est faite cote API. Reutilise les
/// helpers partages du mode vote.
async fn apply_web_resolution(ctx: &Context, action_id: &str, applied_action: &str) {
    if !matches!(
        applied_action,
        "prevention" | "warn" | "delete" | "mute" | "ban"
    ) {
        return; // "ignore" ou inconnu : rien a appliquer.
    }
    // Idempotence : le consumer Redis (stream group + ack) peut redelivrer
    // l'event. On ne (re)applique la sanction qu'une fois par review.
    if !super::claim_once(ctx, &format!("webres:{action_id}")).await {
        info!(
            action_id,
            "Event web-resolve deja applique (redelivrance ignoree)"
        );
        return;
    }
    let (api, grpc) = {
        let data = ctx.data.read().await;
        match (
            data.get::<ApiClientKey>(),
            data.get::<crate::shared::grpc_client::GrpcClientKey>(),
        ) {
            (Some(a), Some(g)) => (a.clone(), g.clone()),
            _ => return,
        }
    };
    let review_api = super::api_client::ApiClient::new(grpc);

    let review = match review_api.get_review(action_id).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, action_id, "Echec fetch review (resolution web) : sanction non appliquee");
            return;
        }
    };

    let config = api
        .get_guild_config_for(&review.guild_id, crate::modules::automod::MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let mute_secs =
        BaseApiClient::config_u64(&config, "mute_duration_secs", DEFAULT_MUTE_DURATION_SECS);

    let gid = review
        .guild_id
        .parse::<u64>()
        .ok()
        .map(serenity::model::id::GuildId::new);

    // Action Discord (delete/mute/ban ; warn = pas d'action destructive).
    // La sanction est tracee cote API dans la requete /resolve (source web) ;
    // le bot n'applique ici que l'action Discord (pas de log redondant).
    let _ = super::vote::apply_member_sanction(
        ctx,
        gid,
        &review.channel_id,
        &review.message_id,
        &review.user_id,
        applied_action,
        mute_secs,
    )
    .await;
}

async fn edit_review_card_from_web(
    ctx: &Context,
    action_id: &str,
    applied_action: &str,
    actor_name: &str,
) {
    use serenity::all::{ChannelId, GetMessages, MessageId};

    let grpc = {
        let data_lock = ctx.data.read().await;
        match data_lock.get::<crate::shared::grpc_client::GrpcClientKey>() {
            Some(g) => g.clone(),
            None => return,
        }
    };
    let mappings = match crate::sync::list_action_messages(&grpc, action_id).await {
        Ok(list) => list,
        Err(e) => {
            warn!(error = %e, action_id, "Echec fetch mapping automod_review");
            return;
        }
    };
    let mapping = match mappings.into_iter().find(|m| m.kind == "automod_review") {
        Some(m) => m,
        None => return,
    };

    let channel_id = match mapping.channel_id.parse::<u64>() {
        Ok(v) => ChannelId::new(v),
        Err(_) => return,
    };
    let msg_id_u64 = match mapping.message_id.parse::<u64>() {
        Ok(v) => v,
        Err(_) => return,
    };
    let msg_id = MessageId::new(msg_id_u64);

    let label = match applied_action {
        "prevention" => "Prevention appliquee",
        "warn" => "Avertissement applique",
        "delete" => "Message supprime",
        "mute" => "Mute applique",
        "ban" => "Bannissement valide",
        "ignore" => "Ignore",
        _ => "Action appliquee",
    };

    if let Ok(messages) = channel_id
        .messages(&ctx.http, GetMessages::new().limit(1).around(msg_id))
        .await
    {
        if let Some(original) = messages.into_iter().find(|m| m.id == msg_id) {
            if let Some(existing_embed) = original.embeds.first() {
                let new_embed = serenity::builder::CreateEmbed::from(existing_embed.clone())
                    .color(0x95A5A6)
                    .footer(serenity::builder::CreateEmbedFooter::new(format!(
                        "{} via web par {}",
                        label, actor_name
                    )))
                    .timestamp(serenity::model::Timestamp::now());
                if let Err(e) = channel_id
                    .edit_message(
                        &ctx.http,
                        msg_id,
                        serenity::builder::EditMessage::new()
                            .embed(new_embed)
                            .components(vec![]),
                    )
                    .await
                {
                    warn!(error = %e, %channel_id, %msg_id, "Echec edit carte automod review");
                }
            }
        }
    }

    info!(
        action_id,
        applied_action, actor_name, "Carte automod review editee suite resolution web"
    );
}

/// Sanitise le contenu utilisateur pour l'affichage dans les embeds Discord.
pub(super) fn sanitize_embed_content(content: &str, max_len: usize) -> String {
    let truncated: String = content.chars().take(max_len).collect();
    truncated
        .replace("```", "` ` `")
        .replace("||", "| |")
        .replace("@everyone", "@-everyone")
        .replace("@here", "@-here")
}
