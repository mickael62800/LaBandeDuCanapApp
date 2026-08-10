//! Creation et envoi des cartes de vote (automatique et manuelle).

use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::shared::grpc_client::GrpcClientKey;
use crate::shared::heartbeat::ApiClientKey;

use super::super::api_client::{Action, ApiClient, CreateReviewParams};
use super::super::detectors;
use super::super::review;
use super::cards::edit_aggregated_card;
use super::context::{fetch_context_after, fetch_context_before};
use super::labels::{action_char, action_label, char_to_str};
use super::render::{
    aggregated_vote_embed, render_history_totals, render_votes, secondary_row, vote_buttons,
    vote_embed, VOTES_FIELD,
};

/// Une carte de vote doit toujours proposer au moins le palier `warn`.
/// Les scores et incidents restent inchangés ; c'est seulement la suggestion
/// initiale lorsque le mode humain a demandé une carte avant un seuil d'action.
fn card_suggested_action(action: &Action) -> Action {
    match action {
        Action::None => Action::Warn,
        Action::Warn => Action::Warn,
        Action::Delete => Action::Delete,
        Action::Mute => Action::Mute,
        Action::Kick => Action::Kick,
        Action::Ban => Action::Ban,
    }
}

/// Cree la review en mode vote et poste la carte avec les boutons de vote.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn post_vote_card(
    ctx: &Context,
    msg: &Message,
    suggested_action: &Action,
    reason: &str,
    score: f64,
    flags: &detectors::DetectionFlags,
    review_channel_id: u64,
    deadline_hours: i64,
    context_before: u8,
    thread_enabled: bool,
    aggregate: bool,
    aggregate_window_minutes: i64,
    discussion_enabled: bool,
    detail_url: Option<String>,
    auto_note: Option<String>,
    // `true` si l'auto-protection a déjà journalisé une sanction (anti
    // double-strike à la finalisation, cf. C1).
    already_sanctioned: bool,
) {
    // En moderation humaine, l'API route explicitement chaque signal vers une
    // carte. Un score individuel peut toutefois rester sous le premier seuil
    // de sanction (Action::None). Ne pas abandonner ici : cela cassait
    // l'agregation, donc un membre pouvait enchainer les signaux sans jamais
    // ouvrir ni mettre a jour sa carte. On propose alors le palier minimal
    // (warn) ; les moderateurs restent libres de voter une autre action et la
    // carte agregee affiche le score cumule reel.
    let card_action = card_suggested_action(suggested_action);
    let suggested_action = &card_action;
    let guild_id = msg.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let channel_id = msg.channel_id.to_string();
    let message_id = msg.id.to_string();
    let user_id = msg.author.id.to_string();
    let content_preview = review::sanitize_embed_content(&msg.content, 500);

    let (api, grpc) = {
        let data = ctx.data.read().await;
        match (data.get::<ApiClientKey>(), data.get::<GrpcClientKey>()) {
            (Some(a), Some(g)) => (a.clone(), g.clone()),
            _ => return,
        }
    };
    let review_api = ApiClient::new(grpc);

    // 1. Creer la review en mode vote (avec echeance) pour obtenir son id.
    //    Si `aggregate`, l'API peut fusionner l'incident dans une carte
    //    'voting' ouverte du meme utilisateur -> on edite alors la carte
    //    existante au lieu d'en poster une nouvelle.
    let deadline = chrono::Utc::now()
        + chrono::Duration::hours(
            sentinel_core::domain::entities::moderation::review::automod::clamp_vote_deadline_hours(
                deadline_hours,
            ),
        );
    let suggested_str = char_to_str(action_char(suggested_action));
    let resp = match review_api
        .create_review(CreateReviewParams {
            guild_id: &guild_id,
            channel_id: &channel_id,
            message_id: &message_id,
            user_id: &user_id,
            user_name: &msg.author.name,
            content_preview: &content_preview,
            suggested_action: suggested_str,
            score,
            reason,
            flags: serde_json::json!({
                "spam": flags.spam, "insult": flags.insult,
                "link": flags.link, "phishing": flags.phishing,
            }),
            voting_deadline: Some(deadline.to_rfc3339()),
            aggregate,
            aggregate_window_minutes: Some(aggregate_window_minutes),
            already_sanctioned,
        })
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Echec creation review vote (sync degrade)");
            return;
        }
    };
    let review_id = resp.id.clone();

    // Cas agregation : l'incident a ete fusionne -> on edite la carte existante.
    // Si le message de cette carte a disparu (supprime), on ne `return` pas :
    // on retombe sur le posting normal ci-dessous pour reposter une carte neuve.
    if resp.merged && edit_aggregated_card(ctx, &api, &review_api, &resp).await {
        return;
    }

    // 2. Recuperer le contexte (N messages avant) pour aider les moderateurs.
    let context = fetch_context_before(ctx, msg, context_before).await;

    // 3. Construire la carte. En mode agregation, layout enrichi (incidents).
    let mut embed = if aggregate {
        aggregated_vote_embed(&resp, &[])
    } else {
        vote_embed(
            &user_id,
            &msg.author.name,
            &channel_id,
            score,
            &content_preview,
            reason,
            flags,
            suggested_str,
            &deadline,
            &[],
        )
    };
    if !context.is_empty() {
        embed = embed.field("Contexte (messages precedents)", context, false);
    }
    // Action automatique deja appliquee (raid / phishing / pub / gros flood).
    if let Some(note) = &auto_note {
        embed = embed.field("🚨 Action automatique appliquee", note, false);
    }
    // 2e section : antecedents de moderation du membre (avec dates).
    if let Some(hist) = render_history_totals(ctx, &guild_id, &user_id).await {
        embed = embed.field("📋 Antecedents du membre", hist, false);
    }

    // Bouton lien : clic -> saute directement sur le message dans le salon.
    let msg_url = format!(
        "https://discord.com/channels/{}/{}/{}",
        guild_id, channel_id, message_id
    );
    let link_row = secondary_row(
        &msg_url,
        &review_id,
        discussion_enabled,
        detail_url.as_deref(),
    );

    let builder = serenity::builder::CreateMessage::new()
        .embed(embed)
        .components({
            let mut rows = vote_buttons(&review_id);
            rows.push(link_row);
            rows
        });

    let posted = match serenity::model::id::ChannelId::new(review_channel_id)
        .send_message(&ctx.http, builder)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            error!(error = %e, review_channel = review_channel_id, "Echec envoi carte de vote");
            return;
        }
    };

    // 3. Enregistrer le mapping pour le sync (web + event decided).
    if let Ok(uuid) = uuid::Uuid::parse_str(&review_id) {
        crate::sync::register_action_message(
            review_api.grpc(),
            uuid,
            crate::sync::kinds::AUTOMOD_REVIEW,
            &guild_id,
            &posted.channel_id.to_string(),
            &posted.id.to_string(),
        )
        .await;
    }
    // Fil de discussion attache a la carte (debat des moderateurs).
    if thread_enabled {
        let thread_name = format!("Vote — {}", msg.author.name);
        let thread_name: String = thread_name.chars().take(90).collect();
        if let Err(e) = posted
            .channel_id
            .create_thread_from_message(
                &ctx.http,
                posted.id,
                serenity::builder::CreateThread::new(thread_name).auto_archive_duration(
                    serenity::model::channel::AutoArchiveDuration::ThreeDays,
                ),
            )
            .await
        {
            warn!(error = %e, "Echec creation fil de discussion sur la carte de vote (permission CREATE_PUBLIC_THREADS ?)");
        }
    }

    info!(review_id, "Carte de vote automod postee");
}

/// Variante manuelle : une carte de vote creee par un moderateur via la
/// commande `/card` (et non par la detection automod). Difference cle : on
/// affiche le contexte AVANT **et** APRES le message pour donner le contexte
/// complet de l'echange. Reutilise le meme flux de review/vote/finalisation
/// que la carte automatique (memes boutons `amv:`/`amf:`, meme review en base).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn post_manual_vote_card(
    ctx: &Context,
    msg: &Message,
    suggested_action: &Action,
    reason: &str,
    review_channel_id: u64,
    deadline_hours: i64,
    context_count: u8,
    thread_enabled: bool,
    moderator_name: &str,
    discussion_enabled: bool,
    aggregate: bool,
    detail_url: Option<String>,
) {
    if matches!(suggested_action, Action::None) {
        return;
    }
    let guild_id = msg.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let channel_id = msg.channel_id.to_string();
    let message_id = msg.id.to_string();
    let user_id = msg.author.id.to_string();
    let content_preview = review::sanitize_embed_content(&msg.content, 500);

    let (api, grpc) = {
        let data = ctx.data.read().await;
        match (data.get::<ApiClientKey>(), data.get::<GrpcClientKey>()) {
            (Some(a), Some(g)) => (a.clone(), g.clone()),
            _ => return,
        }
    };
    let review_api = ApiClient::new(grpc);

    // 1. Creer la review en mode vote (memes champs que la carte automod ;
    // score 0 et flags vides car signalement humain, pas IA).
    let deadline = chrono::Utc::now()
        + chrono::Duration::hours(
            sentinel_core::domain::entities::moderation::review::automod::clamp_vote_deadline_hours(
                deadline_hours,
            ),
        );
    let suggested_str = char_to_str(action_char(suggested_action));
    let resp = match review_api
        .create_review(CreateReviewParams {
            guild_id: &guild_id,
            channel_id: &channel_id,
            message_id: &message_id,
            user_id: &user_id,
            user_name: &msg.author.name,
            content_preview: &content_preview,
            suggested_action: suggested_str,
            score: 0.0,
            reason,
            flags: serde_json::json!(
                { "spam": false, "insult": false, "link": false, "phishing": false }
            ),
            voting_deadline: Some(deadline.to_rfc3339()),
            aggregate,
            aggregate_window_minutes: None,
            already_sanctioned: false,
        })
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Echec creation review (carte manuelle)");
            return;
        }
    };
    let review_id = resp.id.clone();

    // Agregation : si l'incident a ete fusionne, on edite la carte existante.
    // Si son message a disparu (supprime), on retombe sur le posting normal
    // ci-dessous pour reposter une carte neuve (mapping upserte).
    if resp.merged && edit_aggregated_card(ctx, &api, &review_api, &resp).await {
        return;
    }

    // 2. Contexte avant ET apres le message cible.
    let before = fetch_context_before(ctx, msg, context_count).await;
    let after = fetch_context_after(ctx, msg, context_count).await;

    // 3. Construire la carte (embed dedie : pas de labels "IA").
    let mut embed = serenity::builder::CreateEmbed::new()
        .title("Signalement manuel -- VOTE des moderateurs")
        .color(0x5865f2)
        .field(
            "Utilisateur",
            format!("<@{}> (`{}`)", user_id, msg.author.name),
            true,
        )
        .field("Salon", format!("<#{}>", channel_id), true)
        .field("Signale par", format!("`{}`", moderator_name), true)
        .field(
            "Message signale",
            format!("```{}```", content_preview),
            false,
        )
        .field("Raison", reason, false)
        .field("Action suggeree", action_label(suggested_str), true)
        .field("Cloture", format!("<t:{}:R>", deadline.timestamp()), true);
    if !before.is_empty() {
        embed = embed.field("Contexte (avant)", before, false);
    }
    if !after.is_empty() {
        embed = embed.field("Contexte (apres)", after, false);
    }
    if let Some(hist) = render_history_totals(ctx, &guild_id, &user_id).await {
        embed = embed.field("📋 Antecedents du membre", hist, false);
    }
    embed = embed
        .field(VOTES_FIELD, render_votes(&[]), false)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Votez la sanction. A l'echeance, un admin finalise.",
        ))
        .timestamp(serenity::model::Timestamp::now());

    let msg_url = format!(
        "https://discord.com/channels/{}/{}/{}",
        guild_id, channel_id, message_id
    );
    let link_row = secondary_row(
        &msg_url,
        &review_id,
        discussion_enabled,
        detail_url.as_deref(),
    );

    let builder = serenity::builder::CreateMessage::new()
        .embed(embed)
        .components({
            let mut rows = vote_buttons(&review_id);
            rows.push(link_row);
            rows
        });

    let posted = match serenity::model::id::ChannelId::new(review_channel_id)
        .send_message(&ctx.http, builder)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            error!(error = %e, review_channel = review_channel_id, "Echec envoi carte manuelle");
            return;
        }
    };

    // 4. Mapping pour le sync (web + event decided), identique a l'automod.
    if let Ok(uuid) = uuid::Uuid::parse_str(&review_id) {
        crate::sync::register_action_message(
            review_api.grpc(),
            uuid,
            crate::sync::kinds::AUTOMOD_REVIEW,
            &guild_id,
            &posted.channel_id.to_string(),
            &posted.id.to_string(),
        )
        .await;
    }
    if thread_enabled {
        let thread_name = format!("Vote — {}", msg.author.name);
        let thread_name: String = thread_name.chars().take(90).collect();
        if let Err(e) = posted
            .channel_id
            .create_thread_from_message(
                &ctx.http,
                posted.id,
                serenity::builder::CreateThread::new(thread_name).auto_archive_duration(
                    serenity::model::channel::AutoArchiveDuration::ThreeDays,
                ),
            )
            .await
        {
            warn!(error = %e, "Echec creation fil sur la carte manuelle");
        }
    }

    info!(review_id, "Carte de vote manuelle postee");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_for_a_signal_below_threshold_starts_at_warn() {
        assert!(matches!(card_suggested_action(&Action::None), Action::Warn));
    }

    #[test]
    fn card_preserves_an_existing_suggested_action() {
        assert!(matches!(card_suggested_action(&Action::Mute), Action::Mute));
    }
}
