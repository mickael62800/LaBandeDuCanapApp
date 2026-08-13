use super::*;
use crate::sentinel::domain::services::moderation::scoring_service::resolve_thresholds;

impl AnalyzeMessageService {
    pub(super) async fn analyze_impl(
        &self,
        cmd: AnalyzeMessageCommand,
    ) -> Result<MessageAnalysis, DomainError> {
        // 1. Charger les règles (cache → DB)
        let rules = match self.cache.get_rules(&cmd.guild_id).await? {
            Some(cached) => cached,
            None => {
                let from_db = self.rule_repo.find_by_guild(&cmd.guild_id).await?;
                if let Err(e) = self.cache.set_rules(&cmd.guild_id, &from_db).await {
                    tracing::warn!(error = %e, guild_id = %cmd.guild_id, "Echec cache set rules");
                }
                from_db
            }
        };

        // 2. Charger la config automod-bot (fusionnee avec l'ancien `ia_config`
        // par la migration 146). On recupere toutes les cles une fois pour
        // partager la lecture avec le scoring, l'inference IA et le bloc
        // "tension de salon" plus bas.
        let automod_entries = match self
            .bot_config_repo
            .get_config(
                &cmd.guild_id,
                crate::sentinel::domain::entities::system::bot_names::AUTOMOD_BOT,
            )
            .await
        {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %cmd.guild_id, "Echec lecture config automod-bot, utilisation defauts");
                vec![]
            }
        };
        let ia_cfg = parse_ia_config_from_bot_config(&automod_entries);
        let text_enabled = ia_cfg.text_enabled;
        let local_onnx_enabled = ia_cfg.local_onnx_enabled;
        let text_threshold = ia_cfg.text_threshold;
        let context_dampening = ia_cfg.context_dampening;
        let context_format = ia_cfg.context_format.clone();
        // Duree de mute configurable (defaut 600s = 10 min). Cle `mute_duration_secs`
        // de la config automod-bot, la meme que celle lue sur le chemin flood
        // (`evaluate_flood`) et non-IA. Le clamp 60s..28j est applique cote bot
        // (cf. `apply_auto_protect`).
        let mute_duration_secs: u64 = automod_entries
            .iter()
            .find(|e| e.config_key == "mute_duration_secs")
            .and_then(|e| e.config_value.parse::<u64>().ok())
            .unwrap_or(600);
        // Modele de scoring (poids par flag + seuils d'action) editable par
        // serveur. Defaut = constantes historiques -> comportement inchange tant
        // que non reconfigure. Source UNIQUE des poids/seuils de baseline.
        let scoring_config = parse_scoring_config(&automod_entries);

        // 3. Scoring basique (flags bot : spam, insult, link, phishing)
        let mut result = ScoringService::score_with_config(
            &cmd.flags,
            &rules,
            &scoring_config,
            mute_duration_secs,
        );
        // Score IA individuel de CE message (0.0 si pas d'inference ou non
        // toxique). Alimente le buffer "tension de salon" apres l'inference.
        let mut ia_score_individual: f64 = 0.0;

        // 4. Inference text IA (sentiment : anger, rage, threat, harassment)

        debug!(
            has_inference = self.inference.is_some(),
            has_tokenizer = self.tokenizer.is_some(),
            text_enabled,
            "Etat inference IA"
        );

        // DeepSeek est un fournisseur distant autonome : il ne depend ni du
        // modele ONNX local ni de son tokenizer. Le garder dans le bloc ONNX
        // rendait le mode IA muet (et sans consommation de tokens) sur les
        // installations qui n'embarquent que DeepSeek.
        if text_enabled && !cmd.content.is_empty() {
            if let Some(ds) = &self.deepseek_service {
                if ds.is_available() {
                    let _permit = self.inference_limiter.acquire().await?;
                    debug!("Lancement analyse DeepSeek Moderation...");
                    let context_texts: Vec<String> = cmd
                        .context_messages
                        .iter()
                        .map(|c| format!("{}: {}", c.username, c.content))
                        .collect();
                    // Cache court par guilde + contenu + contexte : evite les
                    // appels DeepSeek repetes sans reutiliser une analyse dans
                    // une autre conversation ou un autre serveur.
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(cmd.guild_id.as_bytes());
                    hasher.update(cmd.content.as_bytes());
                    for ctx_msg in &context_texts {
                        hasher.update(ctx_msg.as_bytes());
                    }
                    let hash_bytes = hasher.finalize();
                    let cache_key = format!("ai:deepseek:v1:{:x}", hash_bytes);

                    let cached = self
                        .cache
                        .get_json(&cache_key)
                        .await
                        .ok()
                        .flatten()
                        .and_then(|raw| serde_json::from_str(&raw).ok());
                    let analysis = if let Some(analysis) = cached {
                        Ok(analysis)
                    } else {
                        let result = ds.analyze_message(&cmd.content, &context_texts).await;
                        if let Ok(ref analysis) = result {
                            if let Ok(json) = serde_json::to_string(analysis) {
                                let _ = self.cache.set_json(&cache_key, &json, 300).await;
                            }
                        }
                        result
                    };
                    match analysis {
                        Ok(ds_analysis) => {
                            info!(score = ds_analysis.toxicity_score, sentiment = %ds_analysis.sentiment, reason = %ds_analysis.reason, "Reponse DeepSeek Moderation recue");
                            if let Some((ia_score, ia_flags, ds_reason)) = score_deepseek_analysis(
                                &ds_analysis,
                                &rules,
                                text_threshold,
                                &scoring_config,
                            ) {
                                // `toxicity_score` est une confiance 0..1, pas un
                                // poids de moderation. On la pondere donc avec les
                                // memes regles par type que l'ONNX local : une menace
                                // et une insulte ne peuvent pas valoir 0.90 point.
                                let combined_score = result.score + ia_score;
                                let mut fired = cmd.flags.active_flags();
                                for flag in &ia_flags {
                                    if !fired.contains(flag) {
                                        fired.push(flag.clone());
                                    }
                                }
                                let (t_warn, t_delete, t_mute, t_ban) =
                                    resolve_thresholds(&rules, &fired, &scoring_config);
                                let (action, duration) = if combined_score >= t_ban {
                                    (Action::Ban, None)
                                } else if combined_score >= t_mute {
                                    (Action::Mute, Some(mute_duration_secs))
                                } else if combined_score >= t_delete {
                                    (Action::Delete, None)
                                } else if combined_score >= t_warn {
                                    (Action::Warn, None)
                                } else {
                                    (Action::None, None)
                                };
                                let (action, duration) = cap_ia_induced_ban(
                                    action,
                                    duration,
                                    result.score,
                                    t_ban,
                                    mute_duration_secs,
                                );

                                result.score = combined_score;
                                result.action = action;
                                result.duration = duration;
                                result.reason = if result.reason.is_empty() {
                                    ds_reason
                                } else {
                                    format!("{} | {}", result.reason, ds_reason)
                                };
                                ia_score_individual = ia_score;
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "Echec analyse DeepSeek Moderation, fallback ONNX/Regles")
                        }
                    }
                }
            }
        }

        if let (Some(inference), Some(tokenizer)) = (&self.inference, &self.tokenizer) {
            debug!(
                text_available = inference.text_available(),
                tokenizer_available = tokenizer.available(),
                content_empty = cmd.content.is_empty(),
                "Check inference conditions"
            );
            if text_enabled
                && local_onnx_enabled
                && inference.text_available()
                && tokenizer.available()
                && !cmd.content.is_empty()
            {
                // Rate limit inference
                let _permit = self.inference_limiter.acquire().await?;

                debug!("Lancement inference text...");
                let contextual_content =
                    build_contextual_content(&cmd.content, &cmd.context_messages, &context_format);
                let has_context = !cmd.context_messages.is_empty();
                // Timeout 5s pour eviter qu'une inference bloquee ne stalle le hot path.
                let inference_result = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    tokio::task::spawn_blocking({
                        let inf = Arc::clone(inference);
                        let tok = Arc::clone(tokenizer);
                        let rules = rules.clone();
                        let content = contextual_content.clone();
                        move || {
                            let (input_ids, attention_mask) = tok.tokenize(&content)?;
                            let classifications = inf.classify_text(input_ids, attention_mask)?;
                            Ok::<_, String>(score_classifications(
                                &classifications,
                                &rules,
                                text_threshold,
                                &scoring_config,
                            ))
                        }
                    }),
                )
                .await;
                let inference_result = match inference_result {
                    Ok(Ok(r)) => r,
                    Ok(Err(e)) => Err(format!("spawn_blocking: {e}")),
                    Err(_) => Err("Inference text timeout (5s)".to_string()),
                };
                match inference_result {
                    Ok(Some((ia_score, ia_flags, ia_reason))) => {
                        // Attenuer le score IA si du contexte conversationnel est disponible
                        // (reduit les faux positifs sur les blagues entre amis, etc.)
                        let ia_score = if has_context && context_dampening < 1.0 {
                            let dampened = ia_score * context_dampening;
                            debug!(
                                original_ia_score = ia_score,
                                dampened_ia_score = dampened,
                                context_dampening,
                                "Score IA attenue grace au contexte conversationnel"
                            );
                            dampened
                        } else {
                            ia_score
                        };

                        // Combiner : prendre le score le plus eleve
                        let combined_score = result.score + ia_score;

                        info!(
                            bot_score = result.score,
                            ia_score = ia_score,
                            combined = combined_score,
                            ia_flags = %ia_reason,
                            "Scoring combine bot + IA text"
                        );

                        // Recalculer l'action avec le score combine. Les seuils
                        // sont resolus per-flag-type sur les flags reellement
                        // declenches (flags bot + flags IA), pas un minimum
                        // global sur des regles sans rapport.
                        let mut fired = cmd.flags.active_flags();
                        for f in &ia_flags {
                            if !fired.contains(f) {
                                fired.push(f.clone());
                            }
                        }
                        let (t_warn, t_delete, t_mute, t_ban) =
                            resolve_thresholds(&rules, &fired, &scoring_config);

                        let (action, duration) = if combined_score >= t_ban {
                            (Action::Ban, None)
                        } else if combined_score >= t_mute {
                            (Action::Mute, Some(mute_duration_secs))
                        } else if combined_score >= t_delete {
                            (Action::Delete, None)
                        } else if combined_score >= t_warn {
                            (Action::Warn, None)
                        } else {
                            (Action::None, None)
                        };

                        // C5 — borne anti first-message auto-ban (cf.
                        // `cap_ia_induced_ban`).
                        let (action, duration) = cap_ia_induced_ban(
                            action,
                            duration,
                            result.score,
                            t_ban,
                            mute_duration_secs,
                        );

                        // Combiner les raisons
                        let reason = if result.reason.is_empty() {
                            ia_reason
                        } else {
                            format!("{} + {}", result.reason, ia_reason)
                        };

                        result.score = combined_score;
                        result.action = action;
                        result.reason = reason;
                        result.duration = duration;
                        ia_score_individual = ia_score;
                    }
                    Ok(None) => {
                        // Pas de sentiment toxique detecte
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Inference text echouee — scoring bot seul");
                    }
                }
            }
        }

        // 3b. Tension de salon (somme glissante des scores IA des N derniers
        // messages du channel). S'ajoute comme second declencheur : si la
        // tension declenche une action plus severe que l'analyse individuelle,
        // on override. Sinon, l'action individuelle est gardee.
        if let Some(buffer) = self.tension_buffer.as_ref() {
            let tcfg = parse_tension_config(&automod_entries);
            if tcfg.enabled {
                let entry = TensionEntry {
                    score: ia_score_individual,
                    user_id: cmd.user_id.clone(),
                    message_id: cmd.message_id.clone(),
                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                };
                let total =
                    buffer.push_and_sum(&cmd.guild_id, &cmd.channel_id, entry, tcfg.buffer_size);
                let action = ChannelTensionBuffer::decide_action(
                    total,
                    tcfg.threshold_warn,
                    tcfg.threshold_delete,
                    tcfg.threshold_mute,
                );
                if action != TensionAction::None {
                    info!(
                        guild_id = %cmd.guild_id,
                        channel_id = %cmd.channel_id,
                        tension_total = total,
                        tension_action = ?action,
                        "Tension de salon declenchee"
                    );
                    // Toujours exposer la tension dans la raison, y compris
                    // quand l'action individuelle est deja plus severe. Le
                    // bot Atrium depend de ce signal pour apaiser le salon.
                    let tension_reason = format!(
                        "Tension de salon (somme glissante {:.2} sur {} derniers messages)",
                        total, tcfg.buffer_size
                    );
                    result.reason = if result.reason.is_empty() {
                        tension_reason
                    } else {
                        format!("{} + {}", result.reason, tension_reason)
                    };
                    if tension_is_stronger(&result.action, action) {
                        let (new_action, duration) = match action {
                            TensionAction::Mute => (Action::Mute, Some(tcfg.mute_duration_secs)),
                            TensionAction::Delete => (Action::Delete, None),
                            TensionAction::Warn => (Action::Warn, None),
                            TensionAction::None => (Action::None, None),
                        };
                        result.action = new_action;
                        result.duration = duration;
                    }
                    // Vider le buffer apres declenchement pour eviter le
                    // re-trigger immediat au message suivant (laisse la
                    // conversation redescendre).
                    buffer.clear_channel(&cmd.guild_id, &cmd.channel_id);
                }
            }
        }

        // 3bis. Decision de routage (DECIDE = API) : on connait ici la config
        // guild + le score + les flags. Le bot n'aura qu'a EXECUTER.
        let routing = {
            use crate::sentinel::domain::services::moderation::automod_routing::{
                cap_to_allowed_auto_action, decide, RoutingInputs,
            };
            let cfg_str = |k: &str| {
                automod_entries
                    .iter()
                    .find(|e| e.config_key == k)
                    .map(|e| e.config_value.as_str())
            };
            let cfg_bool = |k: &str, d: bool| {
                cfg_str(k)
                    .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
                    .unwrap_or(d)
            };
            let cfg_f64 =
                |k: &str, d: f64| cfg_str(k).and_then(|v| v.parse::<f64>().ok()).unwrap_or(d);
            let cfg_u64 =
                |k: &str, d: u64| cfg_str(k).and_then(|v| v.parse::<u64>().ok()).unwrap_or(d);
            let selective_auto_actions = cfg_bool("auto_actions_selective_enabled", false);
            let auto_warn = cfg_bool("auto_warn_enabled", true);
            let auto_delete = cfg_bool("auto_delete_enabled", true);
            let auto_mute = cfg_bool("auto_mute_enabled", true);
            let auto_kick = cfg_bool("auto_kick_enabled", false);
            let auto_ban = cfg_bool("auto_ban_enabled", false);
            let capped_action = cap_to_allowed_auto_action(
                &result.action,
                selective_auto_actions,
                auto_warn,
                auto_delete,
                auto_mute,
                auto_kick,
                auto_ban,
            );
            if capped_action != result.action {
                result.reason = format!(
                    "{} | Sanction automatique ramenee a {} par la configuration",
                    result.reason,
                    capped_action.as_str()
                );
                result.action = capped_action;
                if matches!(result.action, Action::Mute) {
                    result.duration = Some(mute_duration_secs);
                }
            }
            decide(&RoutingInputs {
                flags: &cmd.flags,
                content: &cmd.content,
                score: result.score,
                action: result.action.clone(),
                human_only: cfg_bool("human_only_enabled", false),
                auto_protect: cfg_bool("auto_protect_enabled", true),
                auto_delete_links: cfg_bool("auto_delete_links_enabled", false),
                selective_auto_actions,
                auto_warn,
                auto_delete,
                auto_mute,
                auto_kick,
                auto_ban,
                ai_review_mode: cfg_bool("ai_review_mode", true),
                review_min_score: cfg_f64("review_min_score", 0.0),
                log_channel_set: cfg_u64("log_channel_id", 0) != 0,
            })
        };

        // 4. Persister l'infraction
        let infraction = Infraction {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            channel_id: cmd.channel_id,
            user_id: cmd.user_id,
            username: cmd.username,
            display_name: None,
            message_id: cmd.message_id,
            content: cmd.content,
            flags: cmd.flags,
            score: result.score,
            action: result.action.clone(),
            reason: result.reason.clone(),
            duration: result.duration,
            created_at: chrono::Utc::now(),
        };

        self.infraction_repo.save(&infraction).await?;

        // 5. Retourner l'analyse + la decision de routage
        Ok(MessageAnalysis {
            action: result.action,
            reason: result.reason,
            score: result.score,
            duration: result.duration,
            route: routing.route,
            auto_action: routing.auto_action,
            severe: routing.severe,
            auto_delete_link: routing.auto_delete_link,
        })
    }
}
