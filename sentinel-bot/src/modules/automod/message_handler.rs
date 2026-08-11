//! Handler du traitement des messages pour automod.
//! Analyse spam / insultes / liens / phishing / flood / caps / unicode / attachments.

use std::time::Instant;

use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::embeds::{moderate_embed, warn_embed};
use crate::shared::heartbeat::ApiClientKey;

use super::api_client::Action;
use super::backend::{analyze_message_images, send_to_backend};
use super::config::{apply_night_mode, build_detector_config, build_embed_colors, is_night_mode};
use super::detectors;
use super::review::send_review_card;
use super::{FloodTrackerKey, ProcessedMessagesKey, SlowmodeTrackerKey};

/// Defaults si l'API ne repond pas
const DEFAULT_FLOOD_MAX_MESSAGES: u64 = 5;
const DEFAULT_FLOOD_WINDOW_SECS: u64 = 10;
const DEFAULT_MUTE_DURATION_SECS: u64 = 3600;

/// Main automod message handler. Called from the sentinel handler's message event.
/// Analyzes messages for spam, insults, links, phishing, flood, caps, etc.
pub(super) async fn process(ctx: &Context, msg: &Message) -> bool {
    // Pas d'automod en messages prives (aucune guild -> rien a moderer).
    if msg.guild_id.is_none() {
        return false;
    }
    // Deduplication : ignorer si deja traite
    {
        let data = ctx.data.read().await;
        if let Some(processed) = data.get::<ProcessedMessagesKey>() {
            let now = Instant::now();
            // Insertion ATOMIQUE : si la cle existait deja, le message a deja
            // ete traite (redelivrance gateway concurrente) -> on sort. Evite
            // un contains_key+insert non atomique qui laissait passer 2 fois.
            if processed.insert(msg.id, now).is_some() {
                return false;
            }
            if processed.len() > 2000 {
                processed.retain(|_, ts| now.duration_since(*ts).as_secs() < 300);
            }
        }
    }

    // Charger la config depuis l'API pour ce guild
    let guild_id = msg.guild_id.map(|id| id.to_string()).unwrap_or_default();
    let config = crate::shared::discord_helpers::guild_config_or_default(
        ctx,
        &guild_id,
        crate::modules::automod::MODULE_BOT_NAME,
    )
    .await;

    if !BaseApiClient::config_bool(&config, "enabled", false) {
        return false;
    }

    let flood_max_messages =
        BaseApiClient::config_u64(&config, "flood_max_messages", DEFAULT_FLOOD_MAX_MESSAGES)
            as usize;
    let flood_window_secs =
        BaseApiClient::config_u64(&config, "flood_window_secs", DEFAULT_FLOOD_WINDOW_SECS);
    let mute_duration_secs =
        BaseApiClient::config_u64(&config, "mute_duration_secs", DEFAULT_MUTE_DURATION_SECS);

    let mut detector_config = build_detector_config(&config);
    let ai_only = BaseApiClient::config_bool(&config, "ai_only_enabled", false)
        && BaseApiClient::config_bool(&config, "text_enabled", true);
    if ai_only {
        // L'IA devient l'autorite pour les comportements conversationnels.
        // On ne coupe pas le phishing ni les fichiers suspects : ce sont des
        // protections de securite qui ne doivent pas attendre un modele.
        detector_config.spam_enabled = false;
        detector_config.caps_enabled = false;
        detector_config.insult_enabled = false;
        detector_config.link_enabled = false;
        detector_config.emoji_spam_enabled = false;
        detector_config.mentions_enabled = false;
        detector_config.unicode_enabled = false;
    }

    let night_mode_enabled = BaseApiClient::config_bool(&config, "night_mode_enabled", false);
    if night_mode_enabled {
        let start = BaseApiClient::config_u64(&config, "night_start_hour", 22) as u8;
        let end = BaseApiClient::config_u64(&config, "night_end_hour", 8) as u8;
        if is_night_mode(start, end) {
            apply_night_mode(&mut detector_config);
        }
    }

    let colors = build_embed_colors(&config);
    let log_channel_id = BaseApiClient::config_u64(&config, "log_channel_id", 0);
    // Modération humaine : si actif, aucune sanction auto -> tout passe par une carte.
    let human_only = BaseApiClient::config_bool(&config, "human_only_enabled", false);
    // Auto-protection des cas severes (raid / phishing / pub Discord / gros flood) :
    // applique une mesure reversible (mute + suppression) MEME en human_only, puis
    // poste toujours la carte. `severe_flood_max_messages` = seuil "gros flood".
    let auto_protect = BaseApiClient::config_bool(&config, "auto_protect_enabled", true);
    // Seuil "gros flood". AUTORITE = API : la decision severe nominale est prise
    // cote serveur (`evaluate_flood`, qui lit `severe_flood_max_messages` avec le
    // defaut server-side `flood_max * 2`). La valeur ci-dessous n'est utilisee
    // QUE par le fallback local quand l'API est injoignable (resilience
    // VOLONTAIRE) : elle reflete simplement la meme config/defaut pour que la
    // degradation reste fidele. Le bot ne DECIDE plus le seuil severe en nominal.
    let severe_flood_max = BaseApiClient::config_u64(
        &config,
        "severe_flood_max_messages",
        (flood_max_messages as u64) * 2,
    )
    .max(flood_max_messages as u64) as usize;
    // Notification DSA au membre (DM motif + droit d'appel) lors d'une action auto.
    let auto_notify_member = BaseApiClient::config_bool(
        &config,
        "sanction_notify_member",
        BaseApiClient::config_bool(&config, "auto_protect_notify_member", true),
    );
    // Mention systematique du droit d'appel sur les messages de sanction (membre).
    let sanction_appeal = BaseApiClient::config_bool(&config, "sanction_appeal_enabled", true);

    // Verifier les salons exclus
    let ignored_channels_str = BaseApiClient::config_or(&config, "ignored_channels", "");
    if !ignored_channels_str.is_empty() {
        let channel_id_str = msg.channel_id.get().to_string();
        let ignored: Vec<&str> = ignored_channels_str.split(',').map(|s| s.trim()).collect();
        if ignored.iter().any(|id| *id == channel_id_str) {
            return false;
        }
    }

    // Verifier les roles ignores
    let ignored_roles_str = BaseApiClient::config_or(&config, "ignored_roles", "");
    if !ignored_roles_str.is_empty() {
        if let Some(member) = &msg.member {
            let ignored: Vec<&str> = ignored_roles_str.split(',').map(|s| s.trim()).collect();
            for role_id_str in &ignored {
                if let Ok(role_id) = role_id_str.parse::<u64>() {
                    if member.roles.iter().any(|r| r.get() == role_id) {
                        return false;
                    }
                }
            }
        }
    }

    let content = &msg.content;

    // Detection pieces jointes suspectes
    let files_review = BaseApiClient::config_bool(&config, "files_review_mode", true);
    // Detection + DECISION "fichier suspect" cote API : le bot envoie les noms
    // de pieces jointes ; la regle (extensions dangereuses + config) et l'action
    // sont arbitrees cote serveur (`evaluate_attachments`). Le bot n'EXECUTE que
    // le verdict. Le gate `suspicious_files_enabled` local evite un appel inutile
    // quand la detection est desactivee.
    if detector_config.suspicious_files_enabled && !msg.attachments.is_empty() {
        let filenames: Vec<String> = msg.attachments.iter().map(|a| a.filename.clone()).collect();

        let (base_opt, grpc_opt) = {
            let data = ctx.data.read().await;
            (
                data.get::<ApiClientKey>().cloned(),
                data.get::<crate::shared::grpc_client::GrpcClientKey>()
                    .cloned(),
            )
        };

        let verdict = match grpc_opt {
            Some(grpc) => {
                let api = super::api_client::ApiClient::new(grpc);
                match api.evaluate_attachments(&guild_id, filenames).await {
                    Ok(v) => Some(v),
                    Err(e) => {
                        warn!(error = %e, "evaluate_attachments gRPC echoue -- fichier laisse passer");
                        None
                    }
                }
            }
            _ => None,
        };

        if let Some(verdict) = verdict {
            if verdict.suspicious {
                info!(user = %msg.author.name, filename = %verdict.filename, "Fichier suspect detecte (verdict API)");

                if (files_review || human_only) && log_channel_id != 0 {
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
                        &verdict.action,
                        &verdict.reason,
                        verdict.score,
                        &flags,
                        log_channel_id,
                        &colors,
                        None,
                        false,
                    )
                    .await;
                } else if human_only {
                    // Modération humaine sans salon de review : on ne supprime pas.
                    warn!(user = %msg.author.name, "Fichier suspect + human_only sans salon review : suppression bloquee");
                } else {
                    let embed = moderate_embed("Fichier suspect supprime")
                        .color(colors.delete)
                        .field("Raison", &verdict.reason, false)
                        .field("Fichier", &verdict.filename, false)
                        .thumbnail(msg.author.face());
                    let builder = serenity::builder::CreateMessage::new().embed(embed);
                    if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                        warn!(error = %e, "Echec envoi notification fichier suspect");
                    }
                    if let Err(e) = msg.delete(&ctx.http).await {
                        warn!(error = %e, message_id = %msg.id, "Echec suppression message fichier suspect");
                    }
                }

                let log_msg = format!(
                    "Fichier suspect -- {} : {}",
                    msg.author.name, verdict.filename
                );
                if let Some(base) = &base_opt {
                    base.send_log("warn", &guild_id, &log_msg);
                }
                return false;
            }
        }
    }

    // Detection flood (clone le tracker pour eviter deadlock sur le RwLock).
    // En mode IA texte exclusif, ce detecteur de cadence est suspendu avec
    // les autres heuristiques comportementales locales.
    if !ai_only {
        let flood_tracker = {
            let data = ctx.data.read().await;
            data.get::<FloodTrackerKey>().cloned()
        };
        // Le lock ctx.data est libere ici

        let (is_flood, flood_count) = if let Some(tracker) = &flood_tracker {
            let key = (msg.channel_id, msg.author.id);
            let now = Instant::now();
            let mut entry = tracker.entry(key).or_default();
            let timestamps = entry.value_mut();
            timestamps.retain(|t| now.duration_since(*t).as_secs() < flood_window_secs);
            timestamps.push(now);
            let count = timestamps.len();
            let flood = count >= flood_max_messages;
            // Drop le entry pour eviter le deadlock avec retain
            drop(entry);
            if tracker.len() > 5000 {
                tracker.retain(|_, ts| {
                    ts.last()
                        .map(|t| now.duration_since(*t).as_secs() < 600)
                        .unwrap_or(false)
                });
            }
            (flood, count)
        } else {
            (false, 0)
        };

        if is_flood {
            // Gros flood : auto-protection immediate (mute + suppression), meme
            // en human_only, puis carte toujours postee avec la note.
            //
            // Decision (severe ou non) prise COTE API : la regle (seuil severe,
            // toggle auto_protect) vit dans la config serveur, plus dans le bot.
            // Le tracker de rate reste local (legitime). Fallback sur le seuil
            // local uniquement si l'API est indisponible (resilience).
            // `flood_card_score` : score affiche sur la carte, fabrique COTE API
            // (`evaluate_flood`). Le fallback local (0.99/0.9) ne sert qu'en cas
            // d'API indisponible.
            let (severe, flood_card_score) = {
                let grpc = {
                    let data = ctx.data.read().await;
                    data.get::<crate::shared::grpc_client::GrpcClientKey>()
                        .cloned()
                };
                let local_fallback = || {
                    let sev = auto_protect && flood_count >= severe_flood_max;
                    (sev, if sev { 0.99 } else { 0.9 })
                };
                match grpc {
                    Some(grpc) => {
                        let api = crate::modules::automod::api_client::ApiClient::new(grpc);
                        let gid = msg.guild_id.map(|g| g.to_string()).unwrap_or_default();
                        match api
                            .evaluate_flood(
                                &gid,
                                &msg.author.id.to_string(),
                                &msg.channel_id.to_string(),
                                flood_count as i32,
                            )
                            .await
                        {
                            Ok((severe, _dur, score)) => (severe, score),
                            Err(e) => {
                                warn!(error = %e, "evaluate_flood gRPC echoue, fallback seuil local");
                                local_fallback()
                            }
                        }
                    }
                    _ => local_fallback(),
                }
            };
            info!(user = %msg.author.name, count = flood_count, severe, "Flood detecte");
            if let Some(tracker) = &flood_tracker {
                tracker.remove(&(msg.channel_id, msg.author.id));
            }

            let (auto_note, auto_sanctioned) = if severe {
                super::backend::apply_auto_protect(
                    ctx,
                    msg,
                    mute_duration_secs,
                    "Gros flood / raid probable",
                    auto_notify_member,
                )
                .await
            } else {
                (None, false)
            };

            let flood_review = BaseApiClient::config_bool(&config, "flood_review_mode", true);
            if (flood_review || severe) && log_channel_id != 0 {
                let flags = detectors::DetectionFlags {
                    spam: true,
                    insult: false,
                    profanity: false,
                    link: false,
                    phishing: false,
                };
                // Cas severe : on suggere Mute (deja applique), sinon Warn.
                let suggested = if severe { Action::Mute } else { Action::Warn };
                let reason = if severe {
                    "Gros flood detecte -- protection automatique appliquee (raid probable)."
                } else {
                    "Flood detecte -- messages envoyes trop rapidement."
                };
                send_review_card(
                    ctx,
                    msg,
                    &suggested,
                    reason,
                    flood_card_score,
                    &flags,
                    log_channel_id,
                    &colors,
                    auto_note,
                    auto_sanctioned,
                )
                .await;
            } else if severe {
                // Severe sans salon de review : protection auto deja appliquee.
                // On poste une card pour que l'admin voie qui/pourquoi.
                info!(user = %msg.author.name, "Gros flood protege automatiquement (pas de salon de review)");
                if auto_note.is_some() {
                    super::backend::post_auto_mute_notice(
                        ctx,
                        msg,
                        "Gros flood / raid probable",
                        mute_duration_secs,
                        log_channel_id,
                    )
                    .await;
                }
            } else {
                let embed = warn_embed("Avertissement AutoMod")
                    .color(colors.warn)
                    .field(
                        "Raison",
                        "Merci de ne pas envoyer autant de messages aussi rapidement.",
                        false,
                    )
                    .thumbnail(msg.author.face());
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                    warn!(error = %e, "Echec envoi avertissement flood");
                }

                let flags = detectors::DetectionFlags {
                    spam: true,
                    insult: false,
                    profanity: false,
                    link: false,
                    phishing: false,
                };
                let ctx_max_msgs =
                    BaseApiClient::config_u64(&config, "context_max_messages", 3) as u8;
                let ctx_max_chars =
                    BaseApiClient::config_u64(&config, "context_max_chars", 200) as usize;
                let ctx_clone = ctx.clone();
                let msg_clone = msg.clone();
                tokio::spawn(async move {
                    // Routage decide cote serveur.
                    send_to_backend(
                        &ctx_clone,
                        &msg_clone,
                        flags,
                        mute_duration_secs,
                        log_channel_id,
                        &colors,
                        ctx_max_msgs,
                        ctx_max_chars,
                        human_only,
                        auto_notify_member,
                        sanction_appeal,
                    )
                    .await;
                });
            }
            return false;
        }
    }

    // Detection caps
    if detector_config.caps_enabled
        && detectors::spam::detect_caps(content, detector_config.caps_threshold_chars)
    {
        info!(user = %msg.author.name, "Caps detecte");
        let caps_review = BaseApiClient::config_bool(&config, "caps_review_mode", true);
        if caps_review && log_channel_id != 0 {
            let flags = detectors::DetectionFlags {
                spam: true,
                insult: false,
                profanity: false,
                link: false,
                phishing: false,
            };
            // Score affiche : fabrique COTE API (`evaluate_caps`). Le bot ne
            // l'invente plus (avant : 0.8 code en dur). Fallback local 0.8
            // uniquement si l'API est injoignable (resilience VOLONTAIRE).
            let caps_card_score = {
                let grpc = {
                    let data = ctx.data.read().await;
                    data.get::<crate::shared::grpc_client::GrpcClientKey>()
                        .cloned()
                };
                match grpc {
                    Some(grpc) => {
                        let api = crate::modules::automod::api_client::ApiClient::new(grpc);
                        let gid = msg.guild_id.map(|g| g.to_string()).unwrap_or_default();
                        match api.evaluate_caps(&gid).await {
                            Ok(score) => score,
                            Err(e) => {
                                warn!(error = %e, "evaluate_caps gRPC echoue, fallback score local");
                                0.8
                            }
                        }
                    }
                    _ => 0.8,
                }
            };
            send_review_card(
                ctx,
                msg,
                &Action::Warn,
                "Abus de majuscules detecte.",
                caps_card_score,
                &flags,
                log_channel_id,
                &colors,
                None,
                false,
            )
            .await;
        } else {
            let embed = warn_embed("Avertissement AutoMod")
                .color(colors.warn)
                .field(
                    "Raison",
                    "Merci d'ecrire normalement sans tout mettre en majuscules.",
                    false,
                )
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                warn!(error = %e, "Echec envoi avertissement caps");
            }
        }
        // Le caps est traite (carte de review OU avertissement) : on ne relance
        // PAS l'analyse IA sur le meme message (evitait un double traitement =
        // deux cartes / double strike). Comme les branches flood/fichier suspect.
        return false;
    }

    // Slowmode adaptatif
    {
        let adaptive_enabled =
            BaseApiClient::config_bool(&config, "adaptive_slowmode_enabled", false);
        if adaptive_enabled {
            // Seuil >= 1 (un 0 activerait le slowmode des le 1er message) ;
            // secondes bornees a la limite Discord (21600).
            let threshold = (BaseApiClient::config_u64(&config, "adaptive_slowmode_threshold", 15)
                .max(1)) as usize;
            let slowmode_secs = BaseApiClient::config_u64(&config, "adaptive_slowmode_seconds", 5)
                .clamp(1, 21600) as u16;

            let data = ctx.data.read().await;
            if let Some(tracker) = data.get::<SlowmodeTrackerKey>() {
                tracker.record_message(msg.channel_id);
                if tracker.should_activate(msg.channel_id, threshold)
                    && tracker.try_start_activation(msg.channel_id)
                {
                    let edit =
                        serenity::builder::EditChannel::new().rate_limit_per_user(slowmode_secs);
                    if let Err(e) = msg.channel_id.edit(&ctx.http, edit).await {
                        warn!(error = %e, "Impossible d'activer le slowmode adaptatif");
                    } else {
                        info!(channel_id = %msg.channel_id, slowmode_secs, "Slowmode adaptatif active");
                        // Marque le salon comme actif (sans effacer le compteur,
                        // sinon il ne pourrait plus etre desactive -> slowmode
                        // colle a vie).
                        tracker.mark_active(msg.channel_id);
                        // BUG3 : persiste cote API pour survivre a un redemarrage
                        // (best-effort, hors chemin critique).
                        if let Some(grpc) = data.get::<crate::shared::grpc_client::GrpcClientKey>()
                        {
                            let grpc = grpc.clone();
                            let gid = guild_id.clone();
                            let cid = msg.channel_id.to_string();
                            tokio::spawn(async move {
                                super::api_client::persist_slowmode(&grpc, &gid, &cid).await;
                            });
                        }
                    }
                    tracker.finish_activation(msg.channel_id);
                }
                if tracker.tracked_channels() > 500 {
                    tracker.cleanup();
                }
            }
        }
    }

    // Analyse locale (spam, insulte, lien, phishing)
    let flags = detectors::analyze(content, &detector_config);

    if flags.spam || flags.insult || flags.link || flags.phishing {
        info!(
            user = %msg.author.name,
            spam = flags.spam, insult = flags.insult, link = flags.link, phishing = flags.phishing,
            "Message flagge localement"
        );
    }

    let ia_text_enabled = BaseApiClient::config_bool(&config, "text_enabled", true);
    let should_analyze =
        flags.spam || flags.insult || flags.link || flags.phishing || ia_text_enabled;

    if !should_analyze {
        return false;
    }

    let context_max_messages = BaseApiClient::config_u64(&config, "context_max_messages", 3) as u8;
    let context_max_chars = BaseApiClient::config_u64(&config, "context_max_chars", 200) as usize;

    let ctx_clone = ctx.clone();
    let msg_clone = msg.clone();
    let vision_enabled = BaseApiClient::config_bool(&config, "vision_enabled", true);
    // Analyse texte : le ROUTAGE (carte/auto/rien + severe + suppression
    // de lien) est decide cote serveur. Le bot execute la decision.
    // `human_only` n'est conserve que pour le fallback "backend injoignable".
    send_to_backend(
        &ctx_clone,
        &msg_clone,
        flags,
        mute_duration_secs,
        log_channel_id,
        &colors,
        context_max_messages,
        context_max_chars,
        human_only,
        auto_notify_member,
        sanction_appeal,
    )
    .await;

    // Analyse image : si le message contient des images, les analyser via l'API.
    if vision_enabled {
        analyze_message_images(
            &ctx_clone,
            &msg_clone,
            mute_duration_secs,
            log_channel_id,
            &colors,
        )
        .await;
    }

    // Si le message a été supprimé pendant l'analyse, l'API renverra une erreur HTTP NotFound (404)
    msg.channel_id.message(&ctx.http, msg.id).await.is_err()
}
