use serenity::model::channel::Message;
use serenity::model::event::MessageUpdateEvent;
use serenity::model::id::{ChannelId, GuildId, MessageId};
use serenity::prelude::*;

use crate::shared::embeds::{critical_embed, info_embed, moderate_embed};
use crate::shared::grpc_client::{grpc_err_to_string, GrpcClientKey};
use platform_proto::sentinel::audit::v1 as proto_audit;

use super::MessageCacheKey;
use super::{audit_event, watched_users};
use super::{log, post_to_channel, send_event};

/// Formate un contenu de message pour un field embed : tronque a `max`,
/// neutralise les mentions de masse et les blocs ``` pour eviter le bris de
/// rendu. Retourne un placeholder si vide.
fn fmt_block(content: &str, max: usize) -> String {
    let trimmed: String = content.chars().take(max).collect();
    let safe = trimmed
        .replace("```", "` ` `")
        .replace("@everyone", "@\u{200b}everyone")
        .replace("@here", "@\u{200b}here");
    let safe = if content.chars().count() > max {
        format!("{safe}…")
    } else {
        safe
    };
    if safe.trim().is_empty() {
        "*(vide / pièce jointe / embed)*".to_string()
    } else {
        format!("```{safe}```")
    }
}

/// Salons de log message : cle dediee puis fallback log_channel_id (gere par
/// post_to_channel).
const MESSAGE_LOG_KEYS: &[&str] = &["message_log_channel_id"];

/// Helper : construit et envoie un embed d'anomalie dans anomaly_channel_id.
async fn post_anomaly_embed(
    ctx: &Context,
    guild_id: &str,
    anomaly_type: &str,
    count: usize,
    window_secs: u64,
    context_info: &str,
) {
    let embed = critical_embed(format!("ANOMALIE -- {}", anomaly_type))
        .field("Count", count.to_string(), true)
        .field("Fenetre", format!("{}s", window_secs), true)
        .description(format!(
            "Un pattern anormal a ete detecte sur la guild.\n{}",
            context_info
        ))
        .timestamp(serenity::model::Timestamp::now())
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Audit | Sentinel -- Urgence",
        ));
    post_to_channel(ctx, guild_id, &["anomaly_channel_id"], embed).await;
}

pub async fn handle_delete(
    ctx: &Context,
    channel_id: ChannelId,
    message_id: MessageId,
    guild_id: Option<GuildId>,
) {
    let gid = match guild_id {
        Some(g) => g,
        None => return,
    };
    let gid_str = gid.to_string();

    let channel_name = super::resolve_channel_name(ctx, channel_id).await;
    let chan_label = channel_name.clone().unwrap_or_else(|| "?".to_string());

    // Chercher dans le cache (on libere le lock aussitot).
    let cached = {
        let data = ctx.data.read().await;
        data.get::<MessageCacheKey>()
            .and_then(|cache| cache.remove(gid, message_id))
    };

    // Suppression d'un message de bot : on n'audite pas (pas de log Discord, pas
    // de tracking).
    if cached.as_ref().map(|c| c.is_bot).unwrap_or(false) {
        return;
    }

    // Qui a supprime ? Discord ne l'indique PAS dans l'event MESSAGE_DELETE :
    // on interroge l'audit log. Absent si l'auteur a supprime son propre
    // message (Discord ne cree pas d'entree dans ce cas).
    let author_id_u64 = cached
        .as_ref()
        .and_then(|c| c.author_id.parse::<u64>().ok());
    let deleter = resolve_deleter(ctx, gid, channel_id, author_id_u64).await;

    let (mut log_msg, mut details) = match &cached {
        Some(c) => {
            let preview = if c.content.chars().count() > 100 {
                format!("{}...", c.content.chars().take(100).collect::<String>())
            } else {
                c.content.clone()
            };
            (
                format!(
                    "Message de {} supprime dans #{} : \"{}\"",
                    c.author_name, chan_label, preview
                ),
                serde_json::json!({
                    "author_id": c.author_id,
                    "author_name": c.author_name,
                    "content": c.content,
                }),
            )
        }
        None => (
            format!("Message {} supprime dans #{}", message_id, chan_label),
            serde_json::json!({}),
        ),
    };

    // Enrichit avec le suppresseur si identifie via l'audit log.
    if let Some((del_id, del_name)) = &deleter {
        log_msg.push_str(&format!(" — supprime par **{del_name}**"));
        if let Some(obj) = details.as_object_mut() {
            obj.insert("deleted_by".into(), serde_json::json!(del_id));
            obj.insert("deleted_by_name".into(), serde_json::json!(del_name));
        }
    }

    log(ctx, "warn", &gid_str, &log_msg).await;

    let mut evt = audit_event::simple(gid_str.clone(), "message_delete")
        .with_channel(channel_id, channel_name)
        .with_details(details);
    evt.target_id = Some(message_id.to_string());
    if let Some(c) = &cached {
        evt.actor_id = Some(c.author_id.clone());
        evt.actor_name = Some(c.author_name.clone());
    }

    send_event(ctx, evt).await;

    // Embed dans le salon de logs Discord (message_log_channel_id -> log_channel_id).
    {
        let mut embed = moderate_embed("🗑️ Message supprimé").field(
            "Salon",
            format!("<#{}>", channel_id),
            true,
        );
        if let Some(c) = &cached {
            embed = embed
                .field(
                    "Auteur",
                    format!("<@{}> (`{}`)", c.author_id, c.author_name),
                    true,
                )
                .field("Contenu", fmt_block(&c.content, 1000), false);
        } else {
            embed = embed.field(
                "Message",
                format!("`{}` (contenu hors cache)", message_id),
                false,
            );
        }
        embed = match &deleter {
            Some((del_id, _)) => embed.field("Supprimé par", format!("<@{del_id}>"), true),
            None => embed.field("Supprimé par", "l'auteur lui-même (ou inconnu)", true),
        };
        embed = embed.timestamp(serenity::model::Timestamp::now());
        post_to_channel(ctx, &gid_str, MESSAGE_LOG_KEYS, embed).await;
    }

    // Surveillance : tracker la suppression si l'auteur est surveille
    if let Some(c) = &cached {
        watched_users::track_activity(
            ctx,
            &gid_str,
            &c.author_id,
            "message_deleted",
            Some(&channel_id.to_string()),
            Some(&chan_label),
            Some(&c.content),
            serde_json::json!({"message_id": message_id.to_string()}),
        )
        .await;
    }

    // Anomaly detection (delete pattern). DECISION server-side.
    let alert_opt = super::super::detect_anomaly(ctx, &gid_str, "delete", 1).await;
    if let Some(alert) = alert_opt {
        if !crate::shared::discord_helpers::is_feature_enabled(
            ctx,
            &gid_str,
            "audit-bot",
            "anomaly_enabled",
            true,
        )
        .await
        {
            return;
        }

        log(
            ctx,
            "error",
            &gid_str,
            &format!(
                "ANOMALIE : {} ({} en {}s)",
                alert.anomaly_type, alert.count, alert.window_secs
            ),
        )
        .await;

        post_anomaly_embed(
            ctx,
            &gid_str,
            &alert.anomaly_type,
            alert.count,
            alert.window_secs,
            &format!("Dernier salon : <#{}>", channel_id),
        )
        .await;

        send_event(
            ctx,
            audit_event::simple(gid_str.clone(), "anomaly_detected").with_details(
                serde_json::json!({
                    "anomaly_type": alert.anomaly_type,
                    "count": alert.count,
                    "window_secs": alert.window_secs,
                }),
            ),
        )
        .await;
    }
}

pub async fn handle_update(
    ctx: &Context,
    old: Option<Message>,
    _new: Option<Message>,
    event: MessageUpdateEvent,
) {
    let guild_gid = match event.guild_id {
        Some(g) => g,
        None => return,
    };
    let gid = guild_gid.to_string();

    // Ignorer les messages edites par des bots
    if event.author.as_ref().map(|a| a.bot).unwrap_or(false) {
        return;
    }

    let author_id = event.author.as_ref().map(|a| a.id.to_string());
    let author_name = event.author.as_ref().map(|a| a.name.clone());
    let new_content = event.content.clone().unwrap_or_default();
    let mut old_content = old.as_ref().map(|m| m.content.clone()).unwrap_or_default();

    // Fallback 1 : cache audit en RAM (contenu de TOUS les messages non-bot,
    // alimente par `cache_message` a chaque message). C'est la source
    // principale du "avant" — le cache serenity (`old`) est souvent vide.
    if old_content.is_empty() {
        let data = ctx.data.read().await;
        if let Some(cache) = data.get::<MessageCacheKey>() {
            if let Some(cached) = cache.get(guild_gid, event.id) {
                if !cached.content.is_empty() {
                    old_content = cached.content;
                }
            }
        }
    }

    // Fallback 2 : si le cache RAM serenity n'avait pas l'ancien message,
    // on tente une lookup DB via /api/user-activity/{guild}/by-message/{msg_id}.
    // Permet de retrouver l'ancien contenu meme apres restart du bot ou
    // pour les messages anciens hors cache.
    if old_content.is_empty() {
        let grpc = ctx.data.read().await.get::<GrpcClientKey>().cloned();
        if let Some(grpc) = grpc {
            let req = proto_audit::GetActivityByMessageRequest {
                guild_id: gid.to_string(),
                message_id: event.id.to_string(),
            };
            match crate::grpc_call!(&grpc, audit, get_activity_by_message, req) {
                Ok(resp) => {
                    if let Some(c) = resp.content {
                        if !c.is_empty() {
                            old_content = c;
                        }
                    }
                }
                Err(e) => tracing::warn!(
                    error = %e,
                    message_id = %event.id,
                    "Echec fallback DB pour old_content"
                ),
            }
        }
    }

    let name = author_name.as_deref().unwrap_or("?");
    log(
        ctx,
        "info",
        &gid,
        &format!(
            "{} a modifie un message -- avant: \"{}\" | apres: \"{}\"",
            name,
            if old_content.is_empty() {
                "(inconnu)"
            } else {
                &old_content
            },
            new_content
        ),
    )
    .await;

    let mut evt = audit_event::simple(gid.clone(), "message_edit")
        .with_channel(event.channel_id, None)
        .with_details(serde_json::json!({
            "old_content": old_content,
            "new_content": new_content,
        }));
    evt.target_id = Some(event.id.to_string());
    evt.actor_id = author_id;
    evt.actor_name = author_name;

    send_event(ctx, evt).await;

    // Embed dans le salon de logs Discord, AVANT / APRES. On n'envoie l'embed
    // que si le contenu a reellement change (Discord declenche aussi un update
    // sur l'unfurl d'embed, l'epinglage, etc. -> sinon on spammerait le salon).
    if !new_content.is_empty() && new_content != old_content {
        let url = format!(
            "https://discord.com/channels/{}/{}/{}",
            gid, event.channel_id, event.id
        );
        let (a_id, a_name) = event
            .author
            .as_ref()
            .map(|a| (a.id.to_string(), a.name.clone()))
            .unwrap_or_else(|| ("?".to_string(), "?".to_string()));
        let embed = info_embed("✏️ Message modifié")
            .field("Auteur", format!("<@{}> (`{}`)", a_id, a_name), true)
            .field("Salon", format!("<#{}>", event.channel_id), true)
            .field(
                "Avant",
                fmt_block(
                    if old_content.is_empty() {
                        "(inconnu)"
                    } else {
                        &old_content
                    },
                    1000,
                ),
                false,
            )
            .field("Après", fmt_block(&new_content, 1000), false)
            .field("Lien", format!("[Aller au message]({url})"), false)
            .timestamp(serenity::model::Timestamp::now());
        post_to_channel(ctx, &gid, MESSAGE_LOG_KEYS, embed).await;
    }

    // Met a jour le cache audit avec le nouveau contenu : une edition
    // ULTERIEURE du meme message affichera correctement ce texte comme "avant".
    if !new_content.is_empty() {
        if let Some(author) = event.author.as_ref() {
            let data = ctx.data.read().await;
            if let Some(cache) = data.get::<MessageCacheKey>() {
                cache.store(
                    guild_gid,
                    event.id,
                    crate::modules::audit::message_cache::CachedMessage {
                        author_id: author.id.to_string(),
                        author_name: author.name.clone(),
                        content: new_content.clone(),
                        channel_id: event.channel_id.to_string(),
                        is_bot: false,
                    },
                );
            }
        }
    }

    // Surveillance : tracker l'edition si l'auteur est surveille
    if let Some(ref author) = event.author {
        watched_users::track_activity(
            ctx,
            &gid,
            &author.id.to_string(),
            "message_edited",
            Some(&event.channel_id.to_string()),
            None,
            Some(&new_content),
            serde_json::json!({"old_content": old_content, "message_id": event.id.to_string()}),
        )
        .await;
    }
}

pub async fn handle_delete_bulk(
    ctx: &Context,
    channel_id: ChannelId,
    multiple_deleted: Vec<MessageId>,
    guild_id: Option<GuildId>,
) {
    let gid = match guild_id {
        Some(g) => g,
        None => return,
    };
    let gid_str = gid.to_string();

    let count = multiple_deleted.len();
    let channel_name = super::resolve_channel_name(ctx, channel_id).await;
    let chan_label = channel_name.as_deref().unwrap_or("?");

    log(
        ctx,
        "error",
        &gid_str,
        &format!("Purge : {} messages supprimes dans #{}", count, chan_label),
    )
    .await;

    send_event(
        ctx,
        audit_event::simple(gid_str.clone(), "message_delete_bulk")
            .with_channel(channel_id, channel_name)
            .with_details(serde_json::json!({
                "count": count,
                "message_ids": multiple_deleted.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            })),
    )
    .await;

    // Anomaly : compter comme N deletes en un seul appel (increment=count).
    // DECISION server-side : l'API rejoue la fenetre event par event et
    // s'arrete au premier franchissement de seuil.
    let alert_opt = super::super::detect_anomaly(ctx, &gid_str, "delete", count).await;

    if let Some(alert) = alert_opt {
        if !crate::shared::discord_helpers::is_feature_enabled(
            ctx,
            &gid_str,
            "audit-bot",
            "anomaly_enabled",
            true,
        )
        .await
        {
            return;
        }

        log(
            ctx,
            "error",
            &gid_str,
            &format!(
                "ANOMALIE : {} ({} en {}s)",
                alert.anomaly_type, alert.count, alert.window_secs
            ),
        )
        .await;

        post_anomaly_embed(
            ctx,
            &gid_str,
            &alert.anomaly_type,
            alert.count,
            alert.window_secs,
            &format!("Purge bulk dans <#{}> ({} messages)", channel_id, count),
        )
        .await;

        send_event(
            ctx,
            audit_event::simple(gid_str.clone(), "anomaly_detected").with_details(
                serde_json::json!({
                    "anomaly_type": alert.anomaly_type,
                    "count": alert.count,
                    "window_secs": alert.window_secs,
                }),
            ),
        )
        .await;
    }
}

/// Determine qui a supprime un message via l'audit log Discord.
///
/// Discord n'indique PAS le suppresseur dans l'event MESSAGE_DELETE. On lit
/// donc l'audit log (action 72) et on correle par salon + auteur cible.
/// Renvoie `None` quand l'auteur a supprime lui-meme son message (Discord ne
/// cree alors aucune entree), ou si le bot n'a pas la permission VIEW_AUDIT_LOG.
///
/// Limites Discord : les suppressions du meme moderateur sur le meme auteur
/// sont agregees (compteur) ; la correlation reste donc heuristique.
async fn resolve_deleter(
    ctx: &Context,
    gid: GuildId,
    channel_id: ChannelId,
    author_id: Option<u64>,
) -> Option<(String, String)> {
    use serenity::model::guild::audit_log::{Action, MessageAction};

    // L'entree audit peut arriver juste apres l'event : petit delai.
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    let logs = gid
        .audit_logs(
            &ctx.http,
            Some(Action::Message(MessageAction::Delete)),
            None,
            None,
            Some(8),
        )
        .await
        .ok()?;

    let entry = logs.entries.into_iter().find(|e| {
        let chan_ok = e
            .options
            .as_ref()
            .and_then(|o| o.channel_id)
            .map(|c| c.get() == channel_id.get())
            .unwrap_or(false);
        let target_ok = match (author_id, e.target_id) {
            (Some(a), Some(t)) => t.get() == a,
            (None, _) => true,
            _ => false,
        };
        chan_ok && target_ok
    })?;

    let executor = entry.user_id;
    let name = executor
        .to_user(&ctx.http)
        .await
        .map(|u| u.name)
        .unwrap_or_else(|_| executor.to_string());
    Some((executor.to_string(), name))
}
