//! Integration backend : envoi au service d'analyse, execution des actions,
//! pipeline d'analyse d'images via ai-worker.

use std::sync::Arc;

use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::{debug, error, info, warn};

use crate::shared::embeds::{critical_embed, moderate_embed};
use crate::shared::heartbeat::ApiClientKey;

use super::api_client::{Action, AnalyzeRequest, ApiClient, MessageMetadata};
use super::config::EmbedColors;
use super::detectors;
use super::review::{sanitize_embed_content, send_review_card};

/// Genere une raison descriptive a partir des flags detecteurs
/// quand le backend n'en retourne pas.
fn build_fallback_reason(flags: &detectors::DetectionFlags) -> String {
    let mut parts = Vec::new();
    if flags.phishing {
        parts.push("lien de phishing");
    }
    if flags.insult {
        parts.push("langage inapproprie");
    }
    if flags.spam {
        parts.push("spam");
    }
    if flags.link {
        parts.push("lien non autorise");
    }
    if parts.is_empty() {
        "Contenu inapproprie detecte".to_string()
    } else {
        format!("Detection : {}", parts.join(", "))
    }
}

/// Categorie partagee avec Atrium lors d'une escalation de tension. Aucun
/// contenu de message ni identifiant de membre ne traverse cette frontiere.
fn calming_kind(flags: &detectors::DetectionFlags) -> &'static str {
    if flags.phishing {
        "phishing"
    } else if flags.spam {
        "flood"
    } else if flags.insult || flags.profanity {
        "toxicity"
    } else if flags.link {
        "unsafe_link"
    } else {
        "mixed"
    }
}

/// Poste une card de notification d'auto-mute (qui / pourquoi / combien de
/// temps) quand l'auto-protection severe a mute SANS qu'une carte de review
/// soit affichee (route None). Sinon l'admin ne voit nulle part la raison.
/// Cible : le salon de logs si configure, sinon le salon du message.
pub(super) async fn post_auto_mute_notice(
    ctx: &Context,
    msg: &Message,
    reason: &str,
    mute_secs: u64,
    log_channel_id: u64,
) {
    let target = if log_channel_id != 0 {
        serenity::model::id::ChannelId::new(log_channel_id)
    } else {
        msg.channel_id
    };
    let mins = (mute_secs / 60).max(1);
    let reason_txt = if reason.trim().is_empty() {
        "Contenu interdit detecte (protection automatique)".to_string()
    } else {
        reason.to_string()
    };
    let embed = critical_embed("\u{1f507} AutoMod — Mute automatique")
        .field(
            "Membre",
            format!("<@{}> (`{}`)", msg.author.id, msg.author.id),
            true,
        )
        .field("Duree", format!("{} min", mins), true)
        .field("Auteur", "AutoMod (protection auto)", true)
        .field("Raison", reason_txt, false)
        .field("Salon", format!("<#{}>", msg.channel_id), true)
        .thumbnail(msg.author.face())
        .timestamp(serenity::model::Timestamp::now());
    if let Err(e) = target
        .send_message(
            &ctx.http,
            serenity::builder::CreateMessage::new().embed(embed),
        )
        .await
    {
        warn!(error = %e, "Echec envoi notice auto-mute");
    }
}

/// Applique une protection automatique reversible : mute (timeout) + suppression
/// du message. Silencieux (pas d'embed dans le salon : c'est la carte de review
/// qui porte l'info). Retourne une note a afficher sur la carte, ou `None` si la
/// guild est absente. Utilise pour raid / phishing / pub Discord / gros flood,
/// y compris quand `human_only` est actif (la decision finale reste humaine).
/// Retourne `(note, sanction_logged)` : `note` = texte à afficher sur la carte
/// (`None` si pas de guild), `sanction_logged` = `true` si une sanction de
/// membre a effectivement été journalisée (mute réussi) — sert à éviter le
/// double comptage de strike lors de la finalisation de la carte (cf. C1).
pub(super) async fn apply_auto_protect(
    ctx: &Context,
    msg: &Message,
    mute_duration_secs: u64,
    reason: &str,
    notify_member: bool,
) -> (Option<String>, bool) {
    let Some(guild_id) = msg.guild_id else {
        return (None, false);
    };
    const MAX_MUTE_SECS: u64 = 28 * 24 * 3600;
    let safe = mute_duration_secs.clamp(60, MAX_MUTE_SECS);

    let now_secs = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(_) => return (None, false),
    };
    let mut mute_ok = false;
    match crate::modules::moderation::role_mute::apply(ctx, guild_id, msg.author.id, safe).await {
        Ok(crate::modules::moderation::role_mute::ApplyResult::Applied) => mute_ok = true,
        Ok(crate::modules::moderation::role_mute::ApplyResult::AlreadyActive) => {
            let _ = msg.delete(&ctx.http).await;
            crate::modules::moderation::appeal_behavior::record(
                ctx,
                msg.channel_id,
                msg.author.id,
                reason,
            )
            .await;
            return (
                Some("Message supprime : le membre est deja mute, echeance inchangee.".to_string()),
                false,
            );
        }
        Ok(crate::modules::moderation::role_mute::ApplyResult::NotConfigured) => {
            if let Ok(dt) = time::OffsetDateTime::from_unix_timestamp(now_secs + safe as i64) {
                let timeout = serenity::model::Timestamp::from(dt);
                match guild_id.member(&ctx.http, msg.author.id).await {
                    Ok(mut member) => match member
                        .disable_communication_until_datetime(&ctx.http, timeout)
                        .await
                    {
                        Ok(_) => mute_ok = true,
                        Err(e) => {
                            warn!(error = %e, user = %msg.author.name, "Auto-protection : echec timeout")
                        }
                    },
                    Err(e) => {
                        warn!(error = %e, user = %msg.author.name, "Auto-protection : membre introuvable")
                    }
                }
            }
        }
        Err(e) => {
            warn!(error = %e, user = %msg.author.name, "Auto-protection : echec role de mute")
        }
    }

    // Suppression du message declencheur (best-effort).
    let _ = msg.delete(&ctx.http).await;

    // Tracabilite : on logge le mute auto comme une sanction de membre dans
    // l'historique de moderation (acteur = le bot lui-meme / AutoMod), au meme
    // titre qu'un mute valide par un humain (compte dans l'escalade).
    if mute_ok {
        let (bot_id, bot_name) = {
            let cu = ctx.cache.current_user();
            (cu.id.to_string(), cu.name.clone())
        };
        super::vote::log_sanction_to_moderation(
            ctx,
            &guild_id.to_string(),
            &msg.channel_id.to_string(),
            &bot_id,
            &bot_name,
            &msg.author.id.to_string(),
            &msg.author.name,
            "mute",
            &format!("Protection automatique AutoMod : {reason}"),
            Some(safe),
        )
        .await;

        crate::shared::discord_helpers::post_sanction_card(
            ctx,
            &guild_id.to_string(),
            crate::shared::discord_helpers::SanctionKind::Mute,
            msg.author.id.get(),
            Some(&msg.author.name),
            "Automod",
            reason,
            Some(&format!("{}min", safe / 60)),
        )
        .await;
    }

    // Notification DSA au membre : motif + droit d'appel (best-effort, le DM
    // peut echouer si le membre a ferme ses MP).
    if notify_member && mute_ok {
        let dm = format!(
            "Une mesure de protection automatique (mute {} min) a ete appliquee a ton encontre.\n\
             Motif : {reason}\n\
             Si tu penses que c'est une erreur, tu peux contester via la commande **/appeal** sur le serveur.",
            safe / 60
        );
        if let Ok(ch) = msg.author.create_dm_channel(&ctx.http).await {
            let _ = ch
                .send_message(
                    &ctx.http,
                    serenity::builder::CreateMessage::new().content(dm),
                )
                .await;
        }
    }

    let mins = safe / 60;
    let note = if mute_ok {
        format!("Mute {mins} min + suppression appliques automatiquement (mesure reversible, tracee). Le membre a ete informe de son droit d'appel (/appeal). A valider/ajuster ci-dessous.")
    } else {
        "Message supprime automatiquement (mute echoue : verifier MODERATE_MEMBERS). A valider/ajuster ci-dessous.".to_string()
    };
    // `sanction_logged = mute_ok` : la sanction n'est journalisée que si le mute
    // a réussi (cf. bloc `if mute_ok` ci-dessus).
    (Some(note), mute_ok)
}

/// Envoie le message au backend pour analyse et execute l'action.
#[allow(clippy::too_many_arguments)]
pub(super) async fn send_to_backend(
    ctx: &Context,
    msg: &Message,
    flags: detectors::DetectionFlags,
    mute_duration_secs: u64,
    log_channel_id: u64,
    colors: &EmbedColors,
    context_max_messages: u8,
    context_max_chars: usize,
    // Conserve uniquement pour le fallback "backend injoignable" (decision
    // serveur indisponible). Le routage nominal est decide cote API.
    human_only: bool,
    notify_member: bool,
    // Ajoute la mention du droit d'appel aux messages de sanction (membre).
    appeal: bool,
) {
    // Recuperer les N derniers messages du canal pour le contexte conversationnel
    let context_messages = if context_max_messages == 0 {
        Vec::new()
    } else {
        match msg
            .channel_id
            .messages(
                &ctx.http,
                serenity::builder::GetMessages::new()
                    .before(msg.id)
                    .limit(context_max_messages),
            )
            .await
        {
            Ok(messages) => messages
                .into_iter()
                .rev() // ordre chronologique
                .filter(|m| !m.author.bot)
                .map(|m| super::api_client::ContextMessage {
                    username: m.author.name.clone(),
                    content: m.content.chars().take(context_max_chars).collect(),
                })
                .collect(),
            Err(e) => {
                tracing::warn!(error = %e, "Echec recuperation contexte canal");
                Vec::new()
            }
        }
    };

    let request = AnalyzeRequest {
        guild_id: msg.guild_id.map(|id| id.to_string()).unwrap_or_default(),
        channel_id: msg.channel_id.to_string(),
        user_id: msg.author.id.to_string(),
        username: msg.author.name.clone(),
        content: msg.content.clone(),
        flags,
        metadata: MessageMetadata {
            message_id: msg.id.to_string(),
            timestamp: msg.timestamp.to_string(),
        },
        context_messages,
    };

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(client) => Arc::clone(client),
        None => {
            error!("BaseApiClient introuvable dans le contexte");
            return;
        }
    };
    let grpc = match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
        Some(g) => Arc::clone(g),
        None => {
            error!("SentinelGrpcClient introuvable dans le contexte");
            return;
        }
    };
    drop(data);

    let api_client = ApiClient::new(grpc);

    match api_client.analyze(&request).await {
        Ok(response) => {
            info!(action = ?response.action, reason = ?response.reason, "Reponse du backend");

            let fallback_reason = build_fallback_reason(&request.flags);
            let effective_reason = response
                .reason
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(fallback_reason);

            // Signal collectif, sans contenu ni identite de membre. Atrium
            // applique son propre cooldown avant de publier le rappel.
            if effective_reason.contains("Tension de salon") {
                base.publish_event(
                    "atrium_calming_requested",
                    serde_json::json!({
                        "guild_id": request.guild_id,
                        "channel_id": request.channel_id,
                        "reason": "channel_tension",
                        "kind": calming_kind(&request.flags),
                    }),
                );
            }

            if response.action != Action::None {
                let guild_id = msg.guild_id.map(|id| id.to_string()).unwrap_or_default();
                let action_label = match &response.action {
                    Action::Warn => "Avertissement",
                    Action::Delete => "Suppression",
                    Action::Mute => "Mute",
                    Action::Kick => "Kick",
                    Action::Ban => "Bannissement",
                    Action::None => "",
                };
                let log_message = format!(
                    "{} -- {} : {}",
                    action_label, msg.author.name, effective_reason,
                );

                base.send_log(
                    if matches!(response.action, Action::Ban) {
                        "error"
                    } else {
                        "warn"
                    },
                    &guild_id,
                    &log_message,
                );
            }

            // DECIDE = API : la decision de routage (carte / auto / rien +
            // severe + suppression de lien) est calculee cote serveur, qui
            // connait la config guild. Le bot se contente d'EXECUTER.
            let score = response.score.unwrap_or(0.0);

            // Cas SEVERE (phishing / pub Discord) : protection auto reversible
            // immediate (mute + suppression + trace + DM appel), meme en human_only.
            let (auto_note, auto_sanctioned) = if response.severe {
                apply_auto_protect(
                    ctx,
                    msg,
                    mute_duration_secs,
                    &effective_reason,
                    notify_member,
                )
                .await
            } else {
                (None, false)
            };

            // Lien non autorise HORS image : suppression auto + tracabilite.
            if response.auto_delete_link {
                if let Err(e) = msg.delete(&ctx.http).await {
                    warn!(error = %e, message_id = %msg.id, "Echec suppression lien non autorise");
                }
                base.send_log(
                    "warn",
                    &request.guild_id,
                    &format!(
                        "Lien non autorise supprime -- {} : {}",
                        msg.author.name, effective_reason
                    ),
                );
                info!(user = %msg.author.name, "Lien non autorise (hors image) supprime automatiquement");
                return;
            }

            use super::api_client::Routing;
            // La protection severe a deja applique un mute reversible ; ne pas
            // executer deux fois la meme sanction sur ce meme message.
            if response.auto_action && !auto_sanctioned {
                if let Err(e) = execute_action(
                    ctx,
                    msg,
                    &response.action,
                    Some(effective_reason.as_str()),
                    mute_duration_secs,
                    colors,
                    appeal,
                )
                .await
                {
                    error!(error = %e, "Erreur lors de l'execution de l'action automatique");
                } else if notify_member {
                    send_sanction_dm(
                        ctx,
                        msg.author.id,
                        &response.action,
                        &effective_reason,
                        mute_duration_secs,
                        appeal,
                        msg.guild_id.map(|id| id.to_string()).as_deref(),
                    )
                    .await;
                }
            }
            match response.route {
                Routing::Card => {
                    send_review_card(
                        ctx,
                        msg,
                        &response.action,
                        &effective_reason,
                        score,
                        &request.flags,
                        log_channel_id,
                        colors,
                        auto_note,
                        auto_sanctioned,
                    )
                    .await;
                }
                Routing::None => {
                    if response.severe {
                        // Protection auto deja appliquee, pas de salon de review :
                        // on poste quand meme une card pour que l'admin voie
                        // QUI a ete mute et POURQUOI (sinon trace invisible).
                        info!(user = %msg.author.name, "Cas severe protege automatiquement (pas de salon de review)");
                        if auto_note.is_some() {
                            post_auto_mute_notice(
                                ctx,
                                msg,
                                &effective_reason,
                                mute_duration_secs,
                                log_channel_id,
                            )
                            .await;
                        }
                    }
                    // Sinon : human_only sans salon, ou rien a faire.
                }
                Routing::Auto => {
                    if response.auto_action {
                        return;
                    }
                    if let Err(e) = execute_action(
                        ctx,
                        msg,
                        &response.action,
                        Some(effective_reason.as_str()),
                        mute_duration_secs,
                        colors,
                        appeal,
                    )
                    .await
                    {
                        error!(error = %e, "Erreur lors de l'execution de l'action");
                    } else if notify_member {
                        send_sanction_dm(
                            ctx,
                            msg.author.id,
                            &response.action,
                            &effective_reason,
                            mute_duration_secs,
                            appeal,
                            msg.guild_id.map(|id| id.to_string()).as_deref(),
                        )
                        .await;
                    }
                }
            }
        }
        Err(e) => {
            error!(error = %e, "Backend injoignable -- action locale par defaut");
            // Modération humaine : pas d'action auto meme en fallback.
            if human_only {
                warn!(user = %msg.author.name, "Backend injoignable + human_only : aucune action auto (suppression bloquee)");
                return;
            }
            // En mode fallback, supprimer les messages flagges (phishing, insulte, spam, lien)
            let reason = if request.flags.phishing {
                Some("Lien suspect detecte.")
            } else if request.flags.insult {
                Some("Langage inapproprie.")
            } else if request.flags.spam {
                Some("Spam detecte.")
            } else if request.flags.link {
                Some("Lien non autorise.")
            } else {
                None
            };

            if let Some(reason_text) = reason {
                let embed = moderate_embed("Message supprime (mode hors-ligne)")
                    .color(colors.delete)
                    .field("Raison", reason_text, false)
                    .thumbnail(msg.author.face());
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                    warn!(error = %e, "Echec envoi notification mode hors-ligne");
                }
                if let Err(e) = msg.delete(&ctx.http).await {
                    warn!(error = %e, message_id = %msg.id, "Echec suppression message mode hors-ligne");
                }
            }
        }
    }
}

/// Execute l'action decidee par le backend. `appeal` ajoute la mention du
/// droit d'appel au message destine au membre (gabarit uniforme).
pub(super) async fn execute_action(
    ctx: &Context,
    msg: &Message,
    action: &Action,
    reason: Option<&str>,
    mute_duration_secs: u64,
    colors: &EmbedColors,
    appeal: bool,
) -> Result<(), serenity::Error> {
    use crate::shared::embeds::sanction_notice;
    let reason_text = reason.unwrap_or("Automod");

    match action {
        Action::None => {}
        Action::Warn => {
            let embed = sanction_notice("warn", reason_text, None, None, appeal)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            msg.channel_id.send_message(&ctx.http, builder).await?;
            info!(user = %msg.author.name, "Avertissement envoye");
        }
        Action::Delete => {
            let content_preview = sanitize_embed_content(&msg.content, 200);
            let embed = sanction_notice("delete", reason_text, None, None, appeal)
                .field(
                    "Contenu original",
                    format!("```{}```", content_preview),
                    false,
                )
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                warn!(error = %e, "Echec envoi notification suppression");
            }
            msg.delete(&ctx.http).await?;
            info!(message_id = %msg.id, "Message supprime");
        }
        Action::Mute => {
            let mute_minutes = mute_duration_secs / 60;
            let embed = sanction_notice("mute", reason_text, Some(mute_minutes), None, appeal)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                warn!(error = %e, "Echec envoi notification mute");
            }
            if let (Some(guild_id_val), Ok(member)) = (msg.guild_id, msg.member(&ctx.http).await) {
                const MAX_MUTE_SECS: u64 = 28 * 24 * 3600;
                let safe_duration = mute_duration_secs.min(MAX_MUTE_SECS);
                match crate::modules::moderation::role_mute::apply(
                    ctx,
                    guild_id_val,
                    member.user.id,
                    safe_duration,
                )
                .await
                {
                    Ok(crate::modules::moderation::role_mute::ApplyResult::Applied) => {
                        info!(user = %msg.author.name, duration_secs = safe_duration, "Utilisateur mute via role");
                    }
                    Err(e) => {
                        warn!(error = %e, user = %msg.author.name, "Echec role de mute automatique")
                    }
                    Ok(crate::modules::moderation::role_mute::ApplyResult::AlreadyActive) => {
                        crate::modules::moderation::appeal_behavior::record(
                            ctx,
                            msg.channel_id,
                            msg.author.id,
                            reason_text,
                        )
                        .await;
                    }
                    Ok(crate::modules::moderation::role_mute::ApplyResult::NotConfigured) => {
                        let mut member = guild_id_val.member(&ctx.http, member.user.id).await?;
                        let secs = match std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                        {
                            Ok(d) => match (d.as_secs() as i64).checked_add(safe_duration as i64) {
                                Some(v) => v,
                                None => {
                                    error!("Overflow timestamp mute (cas improbable)");
                                    return Ok(());
                                }
                            },
                            Err(e) => {
                                error!(error = %e, "Erreur horloge systeme pour le calcul du mute");
                                return Ok(());
                            }
                        };
                        let datetime = match time::OffsetDateTime::from_unix_timestamp(secs) {
                            Ok(dt) => dt,
                            Err(e) => {
                                error!(error = %e, "Timestamp invalide pour le mute");
                                return Ok(());
                            }
                        };
                        let timeout = serenity::model::Timestamp::from(datetime);
                        member
                            .disable_communication_until_datetime(&ctx.http, timeout)
                            .await?;
                        info!(user = %msg.author.name, duration_secs = mute_duration_secs, "Utilisateur mute");
                    }
                }
            }
            if let Err(e) = msg.delete(&ctx.http).await {
                warn!(error = %e, message_id = %msg.id, "Echec suppression message apres mute automod");
            }
        }
        Action::Kick => {
            if let Some(guild_id) = msg.guild_id {
                let embed = sanction_notice("kick", reason_text, None, None, appeal)
                    .thumbnail(msg.author.face());
                if let Err(e) = msg
                    .channel_id
                    .send_message(
                        &ctx.http,
                        serenity::builder::CreateMessage::new().embed(embed),
                    )
                    .await
                {
                    warn!(error = %e, "Echec envoi notification kick");
                }
                guild_id
                    .kick_with_reason(&ctx.http, msg.author.id, reason_text)
                    .await?;
                info!(user = %msg.author.name, "Utilisateur kick automatiquement");
            }
        }
        Action::Ban => {
            if let Some(guild_id) = msg.guild_id {
                let mut embed = critical_embed("Bannissement automatique")
                    .color(colors.ban)
                    .field("Raison", reason_text, false)
                    .thumbnail(msg.author.face());
                if appeal {
                    embed = embed.field(
                        "Contestation",
                        "Tu estimes cette décision injustifiée ? Tu peux la contester via la commande `/appeal`.",
                        false,
                    );
                }
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                    warn!(error = %e, "Echec envoi notification ban");
                }
                guild_id
                    .ban_with_reason(&ctx.http, msg.author.id, 0, reason_text)
                    .await?;
                if let Err(e) = msg.delete(&ctx.http).await {
                    warn!(error = %e, message_id = %msg.id, "Echec suppression message apres ban automod");
                }
                info!(user = %msg.author.name, "Utilisateur banni automatiquement");
            }
        }
    }

    Ok(())
}

/// Informe le membre, par DM et au mieux, d'une sanction effectivement
/// appliquee. L'echec des MP Discord ne doit jamais annuler la sanction.
pub(super) async fn send_sanction_dm(
    ctx: &Context,
    user_id: serenity::model::id::UserId,
    action: &Action,
    reason: &str,
    mute_duration_secs: u64,
    appeal: bool,
    guild_id: Option<&str>,
) {
    let kind = match action {
        Action::Warn => "warn",
        Action::Delete => "delete",
        Action::Mute => "mute",
        Action::Kick => "kick",
        Action::Ban => "ban",
        Action::None => return,
    };
    let duration = matches!(action, Action::Mute).then_some(mute_duration_secs / 60);
    let embed = crate::shared::embeds::sanction_notice(kind, reason, duration, None, appeal);
    match user_id.create_dm_channel(&ctx.http).await {
        Ok(channel) => {
            let mut message = serenity::builder::CreateMessage::new().embed(embed);
            if appeal {
                if let Some(guild_id) = guild_id {
                    message = message.components(vec![
                        crate::modules::moderation::commands::appeal::build_appeal_button(
                            guild_id, "latest",
                        ),
                    ]);
                }
            }
            if let Err(error) = channel.send_message(&ctx.http, message).await {
                warn!(%error, user_id = %user_id, "Echec envoi DM sanction AutoMod");
            }
        }
        Err(error) => {
            warn!(%error, user_id = %user_id, "Impossible d'ouvrir le DM sanction AutoMod")
        }
    }
}

/// Analyse les images attachees a un message via le ai-worker (async).
///
/// Lit la config automod-bot pour les cles vision_* (fusionnees depuis l ex
/// image-bot par la migration 156) :
///   - vision_max_image_size_mb : taille max d une image traitee (defaut 14 Mo)
///   - vision_scan_embeds       : analyse aussi les images dans les embeds
///   - vision_queue_max_retries : nombre de retries sur echec de submission
///
/// Les toggles vision_auto_delete_nsfw / vision_auto_delete_illicit sont
/// appliques COTE API (AnalyzeImageService renvoie l'action deja arbitree).
pub(super) async fn analyze_message_images(
    ctx: &Context,
    msg: &Message,
    mute_duration_secs: u64,
    log_channel_id: u64,
    colors: &EmbedColors,
) {
    let guild_id = msg.guild_id.map(|g| g.to_string()).unwrap_or_default();

    // Lecture de la config automod-bot (ex-image-bot fusionne par la 156).
    let config = crate::shared::discord_helpers::guild_config_or_default(
        ctx,
        &guild_id,
        crate::modules::automod::MODULE_BOT_NAME,
    )
    .await;

    // vision_queue_enabled : kill switch pour la file async ai_jobs.
    // Si false, on skip toute l'analyse (meme si vision_enabled est true).
    let queue_enabled = crate::shared::api_client::BaseApiClient::config_bool(
        &config,
        "vision_queue_enabled",
        true,
    );
    if !queue_enabled {
        return;
    }

    let max_image_size_mb = crate::shared::api_client::BaseApiClient::config_u64(
        &config,
        "vision_max_image_size_mb",
        14,
    );
    let max_image_bytes = (max_image_size_mb as usize) * 1024 * 1024;
    let scan_embeds =
        crate::shared::api_client::BaseApiClient::config_bool(&config, "vision_scan_embeds", true);
    let queue_max_retries = crate::shared::api_client::BaseApiClient::config_u64(
        &config,
        "vision_queue_max_retries",
        3,
    ) as usize;
    // Les toggles vision_auto_delete_nsfw / vision_auto_delete_illicit sont
    // desormais appliques COTE API (AnalyzeImageService) : le bot n'a plus a
    // reinterpreter la `reason` pour re-decider une suppression.

    // Collecte des URLs : pieces jointes + (optionnel) images dans embeds.
    let mut image_urls: Vec<String> = msg
        .attachments
        .iter()
        .filter(|a| {
            a.content_type
                .as_deref()
                .map(|ct| ct.starts_with("image/"))
                .unwrap_or(false)
        })
        .map(|a| a.url.clone())
        .collect();

    if scan_embeds {
        for embed in &msg.embeds {
            if let Some(img) = &embed.image {
                image_urls.push(img.url.clone());
            }
            if let Some(thumb) = &embed.thumbnail {
                image_urls.push(thumb.url.clone());
            }
        }
    }

    if image_urls.is_empty() {
        return;
    }

    let data = ctx.data.read().await;
    let Some(base) = data.get::<crate::shared::heartbeat::ApiClientKey>() else {
        return;
    };

    // Pas de `reqwest::Client::new()` ici : on reutilise le client partage
    // de `BaseApiClient` (connection pooling + timeouts coherents). L'URL
    // de Discord CDN est externe — c'est une lecture HTTP brute legitime,
    // pas un appel API interne.
    let http_client = base.client();
    let api_url = base.base_url().to_string();

    // Fail-safe S3 : si la vision ne peut PAS analyser une image (job non
    // soumis ou resultat absent), on ne laisse pas passer l'image en silence —
    // on poste UNE carte de revue manuelle (au plus une par message).
    let mut vision_unavailable_flagged = false;

    for url in &image_urls {
        // 1. Telecharger l'image depuis Discord (CDN externe) avec un plafond
        //    de taille applique AVANT de tout charger en memoire.
        let mut resp = match http_client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => resp,
            Ok(resp) => {
                warn!(status = %resp.status(), url, "Image download non-success");
                continue;
            }
            Err(e) => {
                warn!(error = %e, url, "Echec download image");
                continue;
            }
        };

        // 1a. Plafond pre-download : si l'en-tete Content-Length depasse la
        //     limite, on n'ouvre meme pas le corps.
        if let Some(len) = resp.content_length() {
            if len as usize > max_image_bytes {
                debug!(
                    size_bytes = len,
                    max_bytes = max_image_bytes,
                    url,
                    "Image > vision_max_image_size_mb (Content-Length), skip sans download"
                );
                continue;
            }
        }

        // 1b. Lecture bornee chunk par chunk : on s'arrete des qu'on depasse la
        //     limite (cas Content-Length absent ou mensonger).
        let mut bytes: Vec<u8> = Vec::new();
        let mut overflow = false;
        let mut read_error = false;
        loop {
            match resp.chunk().await {
                Ok(Some(chunk)) => {
                    if bytes.len() + chunk.len() > max_image_bytes {
                        overflow = true;
                        break;
                    }
                    bytes.extend_from_slice(&chunk);
                }
                Ok(None) => break,
                Err(e) => {
                    warn!(error = %e, url, "Echec lecture bytes image");
                    read_error = true;
                    break;
                }
            }
        }
        if overflow {
            debug!(
                max_bytes = max_image_bytes,
                url, "Image > vision_max_image_size_mb (lecture bornee), skip"
            );
            continue;
        }
        if read_error || bytes.is_empty() {
            continue;
        }

        // 2. Soumettre un job AI via l'API (non-bloquant, queue DB).
        //    Retry jusqu a queue_max_retries fois en cas d echec reseau.
        let payload = serde_json::json!({
            "guild_id": guild_id,
            "channel_id": msg.channel_id.to_string(),
            "user_id": msg.author.id.to_string(),
            "username": msg.author.name,
            "message_id": msg.id.to_string(),
            "image_base64": base64_encode(&bytes),
        });

        let mut job_id: Option<String> = None;
        for attempt in 0..=queue_max_retries {
            // Appel API interne -> on passe par `base.auth(...)` pour
            // ajouter le Bearer token (homogene avec le reste du bot).
            let req = http_client
                .post(format!("{api_url}/api/ai/jobs"))
                .json(&serde_json::json!({
                    "guild_id": guild_id,
                    "job_type": "analyze_image",
                    "input_payload": payload,
                }));
            let submit_resp = match base.auth(req).send().await {
                Ok(r) => r,
                Err(e) => {
                    warn!(error = %e, attempt, "Echec soumission job AI image");
                    continue;
                }
            };

            if submit_resp.status().is_success() {
                if let Ok(v) = submit_resp.json::<serde_json::Value>().await {
                    if let Some(id) = v.get("job_id").and_then(|x| x.as_str()) {
                        job_id = Some(id.to_string());
                        break;
                    }
                }
            } else {
                warn!(status = %submit_resp.status(), attempt, "Job AI image refuse par l'API");
            }
        }

        let Some(job_id) = job_id else {
            warn!("Job AI image abandonne apres {queue_max_retries} retries");
            // Fail-safe : vision indisponible -> carte de revue manuelle.
            if !vision_unavailable_flagged && log_channel_id != 0 {
                post_vision_unavailable_card(ctx, msg, log_channel_id, colors).await;
                vision_unavailable_flagged = true;
            }
            continue;
        };

        // 3. Attendre le resultat via Redis (pub/sub avec timeout 30s).
        let redis_key = format!("ai_result:{job_id}");
        let result = wait_for_ai_result(base, &redis_key).await;

        let Some(result_json) = result else {
            debug!(job_id, "Pas de resultat AI dans le delai (image)");
            // Fail-safe : pas de resultat vision -> carte de revue manuelle.
            if !vision_unavailable_flagged && log_channel_id != 0 {
                post_vision_unavailable_card(ctx, msg, log_channel_id, colors).await;
                vision_unavailable_flagged = true;
            }
            continue;
        };

        // 4. Extraire l'action retournee par l'API.
        let action_str = result_json
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("none");
        let reason = result_json
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("Image detectee");

        // L'action est DEJA arbitree cote API : la vision (`AnalyzeImageService`)
        // applique les toggles `vision_auto_delete_nsfw` / `vision_auto_delete_illicit`
        // et renvoie l'action finale. Le bot n'interprete plus la `reason` : il
        // se contente d'EXECUTER l'action renvoyee.
        let action = match action_str {
            "warn" => Action::Warn,
            "delete" => Action::Delete,
            "mute" => Action::Mute,
            "ban" => Action::Ban,
            _ => Action::None,
        };

        if action == Action::None {
            continue;
        }

        info!(user = %msg.author.name, action = ?action, reason, "Image moderation (via ai-worker)");
        let appeal = crate::shared::api_client::BaseApiClient::config_bool(
            &config,
            "sanction_appeal_enabled",
            true,
        );
        if let Err(e) = execute_action(
            ctx,
            msg,
            &action,
            Some(reason),
            mute_duration_secs,
            colors,
            appeal,
        )
        .await
        {
            warn!(error = %e, "Echec execution action image");
        }
        break;
    }
}

/// Fail-safe vision (S3) : poste une carte de revue manuelle quand une image
/// n'a PAS pu etre analysee (job non soumis ou resultat absent). Carte de revue
/// seule — AUCUNE sanction auto (`already_sanctioned = false`).
async fn post_vision_unavailable_card(
    ctx: &Context,
    msg: &Message,
    log_channel_id: u64,
    colors: &EmbedColors,
) {
    let flags = detectors::DetectionFlags {
        spam: false,
        insult: false,
        profanity: false,
        link: false,
        phishing: false,
    };
    send_review_card(
        ctx,
        msg,
        &Action::Warn,
        "Image non analysée (vision indisponible) — revue manuelle",
        0.0,
        &flags,
        log_channel_id,
        colors,
        None,
        false,
    )
    .await;
}

/// Encode bytes en base64.
fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Attend le resultat d'un job AI via Redis GET (poll 1s, timeout 30s).
async fn wait_for_ai_result(
    _base: &crate::shared::api_client::BaseApiClient,
    redis_key: &str,
) -> Option<serde_json::Value> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let client = redis::Client::open(redis_url.as_str()).ok()?;
    let mut conn = client.get_multiplexed_async_connection().await.ok()?;

    // Poll toutes les secondes pendant 30s max.
    for _ in 0..30 {
        let val: Option<String> = redis::AsyncCommands::get(&mut conn, redis_key).await.ok()?;
        if let Some(json_str) = val {
            return serde_json::from_str(&json_str).ok();
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    None
}
