//! Finalisation admin (`amf:<review_id>`) et execution des sanctions Discord
//! (partagees avec la review 1-clic).

use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::grpc_client::GrpcClientKey;
use crate::shared::heartbeat::ApiClientKey;

use super::super::api_client::{ApiClient, ReviewFacts};
use super::super::review;
use super::cards::{archive_discussion_channel, reply_ephemeral};
use super::labels::action_label;
use super::render::{moderator_facts, reopen_row};
use super::FINALIZE_PREFIX;

/// Handler du bouton admin de finalisation (`amf:<review_id>`).
pub(crate) async fn handle_finalize_button(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    let review_id = match component.data.custom_id.strip_prefix(FINALIZE_PREFIX) {
        Some(r) => r.to_string(),
        None => return,
    };
    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let (api, grpc) = {
        let data = ctx.data.read().await;
        match (data.get::<ApiClientKey>(), data.get::<GrpcClientKey>()) {
            (Some(a), Some(g)) => (a.clone(), g.clone()),
            _ => return,
        }
    };
    let review_api = ApiClient::new(grpc);
    let config = api
        .get_guild_config_for(&guild_id, super::super::MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let admin_role_id = config
        .get("vote_admin_role_id")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|x| *x > 0);

    // Faits Discord -> la regle can_finalize_review est appliquee cote core
    // (full hexa). Un non-admin recevra une erreur (PermissionDenied) au resolve.
    let facts: ReviewFacts = moderator_facts(component, None, admin_role_id).into();

    // Recupere la review (verdict + cible + infraction) depuis l'API.
    let review = match review_api.get_review(&review_id).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, review_id, "Echec fetch review (finalize)");
            reply_ephemeral(ctx, component, "Review introuvable.").await;
            return;
        }
    };
    if review.status != "decided" {
        reply_ephemeral(
            ctx,
            component,
            &format!(
                "Cette review n'est pas finalisable (statut : {}).",
                review.status
            ),
        )
        .await;
        return;
    }
    let decided = review
        .decided_action
        .clone()
        .unwrap_or_else(|| "ignore".to_string());

    // Persiste la resolution cote API (source discord) + faits du demandeur.
    // La sanction de membre est journalisee cote serveur dans le meme appel.
    if let Err(e) = review_api
        .resolve_review(
            &review_id,
            &decided,
            &component.user.id.to_string(),
            &component.user.name,
            facts,
        )
        .await
    {
        warn!(error = %e, review_id, "Echec resolve finalize (non autorise ou deja finalise)");
        reply_ephemeral(
            ctx,
            component,
            "Finalisation impossible (reserve aux admins, ou deja finalise).",
        )
        .await;
        return;
    }

    // La sanction de membre est tracee cote API (dans la meme requete /resolve
    // ci-dessus) -> plus de 2e appel HTTP ici (evite la fenetre "resolu mais
    // non logge"). Le bot se contente d'executer l'action Discord.
    let mute_secs = BaseApiClient::config_u64(
        &config,
        "mute_duration_secs",
        super::super::DEFAULT_MUTE_DURATION_SECS,
    );

    // Execute la sanction Discord (delete/mute/ban).
    let sanction_ok = apply_member_sanction(
        ctx,
        component.guild_id,
        &review.channel_id,
        &review.message_id,
        &review.user_id,
        &decided,
        mute_secs,
    )
    .await;
    // F3 : la review est deja marquee « appliquee » cote API (ordre resolve-first
    // qui protege du double-ban). Si l'execution Discord a echoue, on alerte le
    // moderateur au lieu de laisser la sanction « perdue en silence ».
    if !sanction_ok {
        let _ = component
            .create_followup(
                &ctx.http,
                serenity::builder::CreateInteractionResponseFollowup::new()
                    .content(
                        "⚠️ Sanction **enregistrée**, mais son **exécution Discord a échoué** \
                         (permission manquante ou membre déjà parti). Applique-la manuellement si nécessaire.",
                    )
                    .ephemeral(true),
            )
            .await;
    }

    // BUG #4 : card de sanction pour les decisions humaines du vote (warn/mute/ban),
    // au meme titre que les sanctions manuelles et l'auto-mute automod. Best-effort.
    if let (Some(gid), Some(kind)) = (
        component.guild_id,
        super::super::review::sanction_kind_for(&decided),
    ) {
        if let Ok(uid) = review.user_id.parse::<u64>() {
            let duration_label = if decided == "mute" {
                Some(format!("{}min", mute_secs / 60))
            } else {
                None
            };
            crate::shared::discord_helpers::post_sanction_card(
                ctx,
                &gid.to_string(),
                kind,
                uid,
                Some(&review.user_name),
                &component.user.name,
                if review.reason.is_empty() {
                    "Decision validee par les moderateurs"
                } else {
                    review.reason.as_str()
                },
                duration_label.as_deref(),
            )
            .await;
        }
    }

    // Notice membre (cohérence de ton avec les autres chemins) : on informe le
    // membre en DM de la sanction validée + droit d'appel. Best-effort.
    if BaseApiClient::config_bool(&config, "sanction_notify_member", true)
        && matches!(decided.as_str(), "prevention" | "warn" | "mute" | "ban")
    {
        let appeal = BaseApiClient::config_bool(&config, "sanction_appeal_enabled", true);
        let mins = if decided == "mute" {
            Some(mute_secs / 60)
        } else {
            None
        };
        let embed = crate::shared::embeds::sanction_notice(
            &decided,
            "Décision validée par les modérateurs",
            mins,
            Some(&component.user.name),
            appeal,
        );
        if let Ok(uid) = review.user_id.parse::<u64>() {
            if let Ok(ch) = serenity::model::id::UserId::new(uid)
                .create_dm_channel(&ctx.http)
                .await
            {
                let _ = ch
                    .send_message(
                        &ctx.http,
                        serenity::builder::CreateMessage::new().embed(embed),
                    )
                    .await;
            }
        }
    }

    // Archive le salon de discussion lie (s'il existe) : l'affaire est close.
    archive_discussion_channel(ctx, &review_api, &review_id, &review.user_id).await;

    // Edite la carte : finalise. On conserve les infos utiles (membre +
    // infraction) pour garder une carte close lisible.
    let mut finalized = serenity::builder::CreateEmbed::new()
        .title("AutoMod -- Vote finalise")
        .color(0x2ecc71)
        .field(
            "Membre",
            format!("<@{}> (`{}`)", review.user_id, review.user_name),
            true,
        )
        .field("Sanction", action_label(&decided), true)
        .field("Finalise par", component.user.name.clone(), true)
        .field(
            "Raison",
            if review.reason.is_empty() {
                "—"
            } else {
                review.reason.as_str()
            },
            false,
        )
        .field(
            if review.incident_count > 1 {
                "Dernier message"
            } else {
                "Message"
            },
            format!(
                "```{}```",
                review::sanitize_embed_content(&review.content_preview, 500)
            ),
            false,
        );
    if review.incident_count > 1 {
        finalized = finalized.field("Incidents", review.incident_count.to_string(), true);
    }
    finalized = finalized
        .footer(serenity::builder::CreateEmbedFooter::new(
            "AutoMod Vote | Finalise par un admin",
        ))
        .timestamp(serenity::model::Timestamp::now());
    let _ = component
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::UpdateMessage(
                serenity::builder::CreateInteractionResponseMessage::new()
                    .embed(finalized)
                    .components(vec![reopen_row(&review_id)]),
            ),
        )
        .await;
    info!(review_id, action = %decided, admin = %component.user.name, "Vote automod finalise");
}

/// Enregistre une infraction warn via le module moderation (gRPC log_action),
/// de sorte que le warn issu d'un vote compte dans l'historique et l'escalade
/// au meme titre qu'un /warn manuel. L'admin qui finalise est le "moderateur".
/// Trace une sanction de membre (warn/mute/ban) dans le module moderation via
/// gRPC log_action, pour qu'elle compte dans l'historique et l'escalade au
/// meme titre qu'une commande manuelle. `duration` = duree du mute en secondes.
/// Partage par le vote (finalisation) et la review 1-clic.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn log_sanction_to_moderation(
    ctx: &Context,
    guild_id: &str,
    action_channel_id: &str,
    moderator_id: &str,
    moderator_name: &str,
    target_id: &str,
    target_name: &str,
    action_type: &str,
    reason: &str,
    duration: Option<u64>,
) {
    use crate::modules::moderation::api_client::ModerationAction;
    use crate::modules::moderation::ModerationApiKey;

    // Sanctions de membre tracees dans l'historique (prevention incluse, mais
    // elle ne compte pas dans l'escalade -- gere cote service log_action_with_strike).
    if !matches!(action_type, "prevention" | "warn" | "mute" | "ban") {
        return;
    }

    let mod_api = {
        let data = ctx.data.read().await;
        match data.get::<ModerationApiKey>() {
            Some(a) => a.clone(),
            None => {
                warn!("ModerationApiKey absent : sanction automod non enregistree");
                return;
            }
        }
    };
    let action = ModerationAction {
        guild_id: guild_id.to_string(),
        channel_id: action_channel_id.to_string(),
        moderator_id: moderator_id.to_string(),
        moderator_name: moderator_name.to_string(),
        target_id: target_id.to_string(),
        target_name: target_name.to_string(),
        action_type: action_type.to_string(),
        reason: reason.to_string(),
        gravity: if action_type == "warn" {
            Some("medium".to_string())
        } else {
            None
        },
        duration: if action_type == "mute" {
            duration
        } else {
            None
        },
    };
    if let Err(e) = mod_api.log_action(&action).await {
        warn!(error = %e, target = target_id, action = action_type, "Echec enregistrement sanction automod cote moderation");
    }
}

/// Execute la sanction Discord decidee (delete/mute/ban). Helper partage par le
/// vote (finalisation) et la review 1-clic, pour une seule implementation.
/// `warn`/`ignore` = pas d'action Discord destructive.
/// Execute la sanction Discord. Retourne `true` si l'action a bien ete
/// appliquee (ou n'exigeait rien) ; `false` si l'execution Discord a echoue
/// (permission manquante, membre parti...) — le caller peut alors alerter le
/// moderateur au lieu de laisser la sanction « perdue en silence » alors que la
/// DB la marque appliquee.
#[must_use]
pub(crate) async fn apply_member_sanction(
    ctx: &Context,
    guild_id: Option<serenity::model::id::GuildId>,
    channel_id_str: &str,
    message_id_str: &str,
    user_id_str: &str,
    action: &str,
    mute_secs: u64,
) -> bool {
    let channel_id = match channel_id_str.parse::<u64>() {
        Ok(id) => serenity::model::id::ChannelId::new(id),
        Err(_) => return false,
    };
    match action {
        "delete" | "mute" => {
            if let Ok(mid) = message_id_str.parse::<u64>() {
                let _ = channel_id
                    .delete_message(&ctx.http, serenity::model::id::MessageId::new(mid))
                    .await;
            }
            if action == "mute" {
                let (Some(gid), Ok(uid)) = (guild_id, user_id_str.parse::<u64>()) else {
                    return false;
                };
                let user_id = serenity::model::id::UserId::new(uid);
                match crate::modules::moderation::role_mute::apply(ctx, gid, user_id, mute_secs)
                    .await
                {
                    Ok(crate::modules::moderation::role_mute::ApplyResult::Applied)
                    | Ok(crate::modules::moderation::role_mute::ApplyResult::AlreadyActive) => {
                        return true
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, user_id = %user_id, "Vote AutoMod : echec role de mute");
                        return false;
                    }
                    Ok(crate::modules::moderation::role_mute::ApplyResult::NotConfigured) => {}
                }
                let Ok(mut member) = gid.member(&ctx.http, user_id).await else {
                    return false;
                };
                let until = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0))
                    + mute_secs as i64;
                let Ok(dt) = time::OffsetDateTime::from_unix_timestamp(until) else {
                    return false;
                };
                return member
                    .disable_communication_until_datetime(
                        &ctx.http,
                        serenity::model::Timestamp::from(dt),
                    )
                    .await
                    .is_ok();
            }
            true // "delete" pur : rien de plus a executer
        }
        "ban" => {
            let (Some(gid), Ok(uid)) = (guild_id, user_id_str.parse::<u64>()) else {
                return false;
            };
            {
                let user = serenity::model::id::UserId::new(uid);

                // AutoMod : un ban propose devient un BAN EN SURSIS (appel
                // possible) si le role Sursis est configure. Le ban dur reste
                // pour /ban (scam/raid). On supprime d'abord le message.
                if let Ok(mid) = message_id_str.parse::<u64>() {
                    let _ = channel_id
                        .delete_message(&ctx.http, serenity::model::id::MessageId::new(mid))
                        .await;
                }
                let username = user
                    .to_user(&ctx.http)
                    .await
                    .map(|u| u.name)
                    .unwrap_or_default();
                let sursis = crate::modules::moderation::commands::ban_sursis::apply_sursis(
                    ctx,
                    &gid.to_string(),
                    user,
                    &username,
                    "automod",
                    "AutoMod",
                    "Sanction AutoMod validee",
                )
                .await;
                if sursis.is_some() {
                    // Sursis applique : pas de ban dur -> succes.
                    return true;
                }

                // Fallback : role Sursis non configure -> ban dur.
                // Purge des messages selon le reglage serveur (defaut 7 jours).
                let delete_days: u8 = {
                    let data = ctx.data.read().await;
                    match data.get::<crate::shared::heartbeat::ApiClientKey>() {
                        Some(api) => api
                            .get_guild_config_for(
                                &gid.to_string(),
                                crate::modules::moderation::MODULE_BOT_NAME,
                            )
                            .await
                            .ok()
                            .map(|cfg| {
                                crate::shared::api_client::BaseApiClient::config_u64(
                                    &cfg,
                                    "ban_delete_message_days",
                                    7,
                                ) as u8
                            })
                            .unwrap_or(7),
                        None => 7,
                    }
                };
                match gid
                    .ban_with_reason(
                        &ctx.http,
                        serenity::model::id::UserId::new(uid),
                        delete_days,
                        "Sanction AutoMod validee",
                    )
                    .await
                {
                    Ok(_) => true,
                    Err(e) => {
                        warn!(error = %e, user = user_id_str, "Echec ban (sanction validee) -- permission BAN_MEMBERS ?");
                        false
                    }
                }
            }
        }
        "prevention" => {
            // Cran le plus leger : aucun acte destructif, juste un message public
            // de prevention (la trace est enregistree cote moderation).
            let embed = serenity::builder::CreateEmbed::new()
                .title("Prevention")
                .description(format!(
                    "<@{}> — message de prevention de la moderation. Merci d'ajuster ton comportement.",
                    user_id_str
                ))
                .color(0x3498db);
            let _ = channel_id
                .send_message(
                    &ctx.http,
                    serenity::builder::CreateMessage::new().embed(embed),
                )
                .await;
            true
        }
        // Action inconnue / "ignore" : rien a executer, pas un echec.
        _ => true,
    }
}
