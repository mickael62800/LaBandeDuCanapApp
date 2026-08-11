use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

use crate::domain::entities::ai::message_analysis::MessageAnalysis;
use crate::domain::entities::moderation::infraction::Infraction;
use crate::domain::enums::moderation::action::Action;
use crate::domain::enums::moderation::flag_type::FlagType;
use crate::domain::errors::DomainError;
use crate::domain::services::ai::inference_limiter::InferenceRateLimiter;
use crate::domain::services::moderation::channel_tension::ChannelTensionBuffer;
use crate::domain::services::moderation::channel_tension::TensionAction;
use crate::domain::services::moderation::channel_tension::TensionEntry;
use crate::domain::services::moderation::scoring_service::ScoringConfig;
use crate::domain::services::moderation::scoring_service::ScoringService;
use crate::ports::inbound::ai::analyze_message::AnalyzeMessageCommand;
use crate::ports::inbound::ai::analyze_message::AnalyzeMessageUseCase;
use crate::ports::outbound::ai::inference_service::InferenceService;
use crate::ports::outbound::ai::text_tokenizer::TextTokenizer;
use crate::ports::outbound::moderation::infraction_repository::InfractionRepository;
use crate::ports::outbound::moderation::rule_repository::RuleRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::system::cache::CachePort;
/// Seuil de confiance par defaut (utilise si pas de config per-guild).
const DEFAULT_TEXT_THRESHOLD: f32 = 0.5;

pub struct AnalyzeMessageService {
    rule_repo: Arc<dyn RuleRepository>,
    infraction_repo: Arc<dyn InfractionRepository>,
    cache: Arc<dyn CachePort>,
    /// Repo pour lire la config automod-bot : cles IA (text_enabled,
    /// text_threshold, context_dampening, context_format) + cles tension
    /// de salon (activation + seuils). Anciennement lu depuis la table
    /// dediee `ia_config` ; fusion dans automod-bot via migration 146.
    bot_config_repo: Arc<dyn BotConfigRepository>,
    inference_limiter: Arc<InferenceRateLimiter>,
    inference: Option<Arc<dyn InferenceService>>,
    tokenizer: Option<Arc<dyn TextTokenizer>>,
    deepseek_service: Option<
        Arc<dyn crate::ports::outbound::ai::deepseek_moderation_service::DeepSeekModerationService>,
    >,
    /// Buffer in-memory pour la "tension de salon" (option : si None, la
    /// feature est desactivee quel que soit le contenu de la config).
    tension_buffer: Option<Arc<ChannelTensionBuffer>>,
}

impl AnalyzeMessageService {
    pub fn new(
        rule_repo: Arc<dyn RuleRepository>,
        infraction_repo: Arc<dyn InfractionRepository>,
        cache: Arc<dyn CachePort>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
        inference_limiter: Arc<InferenceRateLimiter>,
    ) -> Self {
        Self {
            rule_repo,
            infraction_repo,
            cache,
            bot_config_repo,
            inference_limiter,
            inference: None,
            tokenizer: None,
            deepseek_service: None,
            tension_buffer: None,
        }
    }

    /// Ajoute le service DeepSeek Moderation au pipeline.
    pub fn with_deepseek(
        mut self,
        deepseek: Arc<
            dyn crate::ports::outbound::ai::deepseek_moderation_service::DeepSeekModerationService,
        >,
    ) -> Self {
        self.deepseek_service = Some(deepseek);
        self
    }

    /// Ajoute l'inference text IA au service d'analyse.
    pub fn with_text_inference(
        mut self,
        inference: Arc<dyn InferenceService>,
        tokenizer: Arc<dyn TextTokenizer>,
    ) -> Self {
        self.inference = Some(inference);
        self.tokenizer = Some(tokenizer);
        self
    }

    /// Ajoute la feature "tension de salon" (buffer glissant + seuils
    /// lus depuis `bot_guild_config` pour `automod-bot`).
    pub fn with_channel_tension(mut self, buffer: Arc<ChannelTensionBuffer>) -> Self {
        self.tension_buffer = Some(buffer);
        self
    }
}

/// Config IA resolue depuis la config `automod-bot` (migration 146).
#[derive(Debug, Clone)]
pub(crate) struct IaConfigValues {
    pub text_enabled: bool,
    pub local_onnx_enabled: bool,
    pub text_threshold: f32,
    pub context_dampening: f64,
    pub context_format: String,
}

impl Default for IaConfigValues {
    fn default() -> Self {
        Self {
            text_enabled: true,
            local_onnx_enabled: true,
            text_threshold: DEFAULT_TEXT_THRESHOLD,
            context_dampening: 0.65,
            context_format: "natural".to_string(),
        }
    }
}

/// Parse les cles IA (`text_enabled`, `text_threshold`, `context_dampening`,
/// `context_format`) depuis la liste des `BotGuildConfig` de `automod-bot`.
/// Fallback sur les defauts si cles absentes/malformees.
pub(crate) fn parse_ia_config_from_bot_config(
    entries: &[crate::domain::entities::system::bot_config::BotGuildConfig],
) -> IaConfigValues {
    let mut cfg = IaConfigValues::default();
    for e in entries {
        match e.config_key.as_str() {
            "text_enabled" => {
                let v = e.config_value.to_ascii_lowercase();
                cfg.text_enabled = matches!(v.as_str(), "true" | "1" | "yes");
            }
            "local_onnx_enabled" => {
                let v = e.config_value.to_ascii_lowercase();
                cfg.local_onnx_enabled = matches!(v.as_str(), "true" | "1" | "yes");
            }
            "text_threshold" => {
                if let Ok(n) = e.config_value.parse::<f32>() {
                    cfg.text_threshold = n.clamp(0.0, 1.0);
                }
            }
            "context_dampening" => {
                if let Ok(n) = e.config_value.parse::<f64>() {
                    cfg.context_dampening = n.clamp(0.0, 1.0);
                }
            }
            "context_format" => {
                let v = e.config_value.as_str();
                if matches!(v, "natural" | "tagged") {
                    cfg.context_format = v.to_string();
                }
            }
            _ => {}
        }
    }
    cfg
}

/// Construit le `ScoringConfig` (poids par flag + seuils d'action) depuis la
/// config `automod-bot`. Chaque clé retombe sur le défaut historique
/// (`ScoringConfig::default()`) si absente/malformée. Source UNIQUE des poids
/// et seuils de baseline — remplace les copies inline dupliquées qui existaient
/// dans les chemins texte et image. Les valeurs sont naturelles (ex. "7"),
/// tolère "7" comme "7.0", et sont bornées à >= 0.
pub(crate) fn parse_scoring_config(
    entries: &[crate::domain::entities::system::bot_config::BotGuildConfig],
) -> ScoringConfig {
    let mut cfg = ScoringConfig::default();
    let get = |key: &str| -> Option<f64> {
        entries
            .iter()
            .find(|e| e.config_key == key)
            .and_then(|e| e.config_value.parse::<f64>().ok())
            .filter(|n| *n >= 0.0)
    };
    if let Some(v) = get("score_weight_spam") {
        cfg.weight_spam = v;
    }
    if let Some(v) = get("score_weight_insult") {
        cfg.weight_insult = v;
    }
    if let Some(v) = get("score_weight_profanity") {
        cfg.weight_profanity = v;
    }
    if let Some(v) = get("score_weight_link") {
        cfg.weight_link = v;
    }
    if let Some(v) = get("score_weight_phishing") {
        cfg.weight_phishing = v;
    }
    if let Some(v) = get("score_weight_nsfw") {
        cfg.weight_nsfw = v;
    }
    if let Some(v) = get("score_weight_illicit") {
        cfg.weight_illicit = v;
    }
    if let Some(v) = get("score_weight_anger") {
        cfg.weight_anger = v;
    }
    if let Some(v) = get("score_weight_rage") {
        cfg.weight_rage = v;
    }
    if let Some(v) = get("score_weight_threat") {
        cfg.weight_threat = v;
    }
    if let Some(v) = get("score_weight_harassment") {
        cfg.weight_harassment = v;
    }
    if let Some(v) = get("score_threshold_warn") {
        cfg.threshold_warn = v;
    }
    if let Some(v) = get("score_threshold_delete") {
        cfg.threshold_delete = v;
    }
    if let Some(v) = get("score_threshold_mute") {
        cfg.threshold_mute = v;
    }
    if let Some(v) = get("score_threshold_ban") {
        cfg.threshold_ban = v;
    }
    cfg
}

/// Config resolue pour la feature "tension de salon".
#[derive(Debug, Clone)]
struct TensionConfig {
    enabled: bool,
    buffer_size: usize,
    threshold_warn: f64,
    threshold_delete: f64,
    threshold_mute: f64,
    mute_duration_secs: u64,
}

impl Default for TensionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            buffer_size: 5,
            threshold_warn: 3.0,
            threshold_delete: 5.0,
            threshold_mute: 7.0,
            mute_duration_secs: 300,
        }
    }
}

/// Parse la config tension depuis la liste des `BotGuildConfig` de
/// `automod-bot`. Defaut si cles absentes/mal formees.
fn parse_tension_config(
    entries: &[crate::domain::entities::system::bot_config::BotGuildConfig],
) -> TensionConfig {
    let mut cfg = TensionConfig::default();
    for e in entries {
        match e.config_key.as_str() {
            "channel_tension_enabled" => {
                let v = e.config_value.to_ascii_lowercase();
                cfg.enabled = matches!(v.as_str(), "true" | "1" | "yes");
            }
            "channel_tension_buffer_size" => {
                if let Ok(n) = e.config_value.parse::<usize>() {
                    if n >= 1 {
                        cfg.buffer_size = n;
                    }
                }
            }
            "channel_tension_threshold_warn" => {
                if let Ok(n) = e.config_value.parse::<f64>() {
                    cfg.threshold_warn = n;
                }
            }
            "channel_tension_threshold_delete" => {
                if let Ok(n) = e.config_value.parse::<f64>() {
                    cfg.threshold_delete = n;
                }
            }
            "channel_tension_threshold_mute" => {
                if let Ok(n) = e.config_value.parse::<f64>() {
                    cfg.threshold_mute = n;
                }
            }
            "channel_tension_mute_duration_secs" => {
                if let Ok(n) = e.config_value.parse::<u64>() {
                    cfg.mute_duration_secs = n;
                }
            }
            _ => {}
        }
    }
    cfg
}

/// Compare la severite d'une action existante et d'une `TensionAction`
/// pour garder la plus forte si les deux declenchent. Retourne `true`
/// si la tension est strictement plus severe.
fn tension_is_stronger(current: &Action, tension: TensionAction) -> bool {
    let sev = |a: &Action| -> u8 {
        match a {
            Action::None => 0,
            Action::Warn => 1,
            Action::Delete => 2,
            Action::Mute => 3,
            Action::Kick => 4,
            Action::Ban => 5,
        }
    };
    let tsev = match tension {
        TensionAction::None => 0,
        TensionAction::Warn => 1,
        TensionAction::Delete => 2,
        TensionAction::Mute => 3,
    };
    tsev > sev(current)
}

#[async_trait]
impl AnalyzeMessageUseCase for AnalyzeMessageService {
    async fn evaluate_flood(
        &self,
        guild_id: &str,
        flood_count: i32,
    ) -> Result<crate::ports::inbound::ai::analyze_message::FloodDecision, DomainError> {
        use crate::ports::inbound::ai::analyze_message::FloodDecision;
        let entries = self
            .bot_config_repo
            .get_config(
                guild_id,
                crate::domain::entities::system::bot_names::AUTOMOD_BOT,
            )
            .await
            .unwrap_or_default();
        let num = |key: &str, default: u64| -> u64 {
            entries
                .iter()
                .find(|e| e.config_key == key)
                .and_then(|e| e.config_value.parse::<u64>().ok())
                .unwrap_or(default)
        };
        let auto_protect = entries
            .iter()
            .find(|e| e.config_key == "auto_protect_enabled")
            .map(|e| {
                let v = e.config_value.to_ascii_lowercase();
                v == "true" || v == "1"
            })
            .unwrap_or(true);
        let flood_max = num("flood_max_messages", 5);
        let severe_max = num("severe_flood_max_messages", flood_max * 2);
        let mute_dur = num("mute_duration_secs", 600);
        let severe = auto_protect && (flood_count.max(0) as u64) >= severe_max;
        // Score de confiance affiche sur la carte : fabrique cote serveur
        // (auparavant code en dur dans le bot : 0.99 severe / 0.9 sinon).
        let score = if severe { 0.99 } else { 0.9 };
        Ok(FloodDecision {
            severe,
            mute_duration_secs: mute_dur as i64,
            score,
        })
    }

    async fn evaluate_attachments(
        &self,
        guild_id: &str,
        filenames: Vec<String>,
    ) -> Result<crate::ports::inbound::ai::analyze_message::AttachmentDecision, DomainError> {
        use crate::ports::inbound::ai::analyze_message::AttachmentDecision;

        // Liste des extensions intrinsequement dangereuses (executables /
        // scripts). Auparavant codee en dur DANS le bot — la regle vit
        // desormais cote serveur.
        const DANGEROUS_EXTENSIONS: &[&str] = &[
            "exe", "bat", "cmd", "scr", "ps1", "vbs", "js", "jar", "com", "pif", "msi", "dll",
            "reg", "hta",
        ];

        let none = || AttachmentDecision {
            suspicious: false,
            action: Action::None,
            reason: String::new(),
            score: 0.0,
            filename: String::new(),
        };

        let entries = self
            .bot_config_repo
            .get_config(
                guild_id,
                crate::domain::entities::system::bot_names::AUTOMOD_BOT,
            )
            .await
            .unwrap_or_default();

        // Toggle `suspicious_files_enabled` (defaut true) : si desactive, aucune
        // piece jointe n'est jugee suspecte.
        let enabled = entries
            .iter()
            .find(|e| e.config_key == "suspicious_files_enabled")
            .map(|e| {
                let v = e.config_value.to_ascii_lowercase();
                matches!(v.as_str(), "true" | "1" | "yes")
            })
            .unwrap_or(true);
        if !enabled {
            return Ok(none());
        }

        // Extensions supplementaires configurees par serveur (CSV).
        let extra: Vec<String> = entries
            .iter()
            .find(|e| e.config_key == "suspicious_file_extensions")
            .map(|e| {
                e.config_value
                    .split(',')
                    .map(|s| s.trim().to_ascii_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        for filename in &filenames {
            let name_lower = filename.to_lowercase();
            let ext = name_lower.rsplit('.').next().unwrap_or("");
            if DANGEROUS_EXTENSIONS.contains(&ext) || extra.iter().any(|e| e == ext) {
                return Ok(AttachmentDecision {
                    suspicious: true,
                    action: Action::Delete,
                    reason: format!("Piece jointe suspecte : {filename}"),
                    score: 1.0,
                    filename: filename.clone(),
                });
            }
        }

        Ok(none())
    }

    async fn evaluate_caps(
        &self,
        guild_id: &str,
    ) -> Result<crate::ports::inbound::ai::analyze_message::CapsDecision, DomainError> {
        use crate::ports::inbound::ai::analyze_message::CapsDecision;
        // Score de confiance affiche pour une detection de CAPS : fabrique cote
        // serveur (auparavant code en dur dans le bot : 0.8). Lu depuis la config
        // guild (`caps_confidence_score`) avec le defaut historique 0.8, borne
        // a [0.0, 1.0]. La detection (forme/longueur) reste locale au bot.
        let entries = self
            .bot_config_repo
            .get_config(
                guild_id,
                crate::domain::entities::system::bot_names::AUTOMOD_BOT,
            )
            .await
            .unwrap_or_default();
        let score = entries
            .iter()
            .find(|e| e.config_key == "caps_confidence_score")
            .and_then(|e| e.config_value.parse::<f64>().ok())
            .map(|v| v.clamp(0.0, 1.0))
            .unwrap_or(0.8);
        Ok(CapsDecision { score })
    }

    async fn analyze(&self, cmd: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError> {
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
                crate::domain::entities::system::bot_names::AUTOMOD_BOT,
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
            use crate::domain::services::moderation::automod_routing::{
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

// run_text_inference supprimee — remplacee par spawn_blocking + timeout dans analyze().

/// Fonction pure : transforme les classifications IA en score, flags et raison.
/// Retourne None si aucun sentiment toxique n'est detecte au-dessus du seuil.
pub fn score_classifications(
    classifications: &[crate::ports::outbound::ai::inference_service::InferenceClassification],
    rules: &[crate::domain::entities::system::rule::Rule],
    threshold: f32,
    scoring_config: &ScoringConfig,
) -> Option<(f64, Vec<FlagType>, String)> {
    let mut detected: Vec<(FlagType, f32)> = Vec::new();

    for c in classifications {
        let flag = match c.label.as_str() {
            // Modele 2 classes : severe = rage + threat agreges.
            // On mappe sur FlagType::Harassment (la plus generique des flags
            // toxiques) pour que le scoring existant fonctionne sans ajouter
            // un nouveau type.
            "severe" if c.confidence >= threshold => Some(FlagType::Harassment),
            // Legacy 5 classes (si vieux modele encore charge).
            "anger" if c.confidence >= threshold => Some(FlagType::Anger),
            "rage" if c.confidence >= threshold => Some(FlagType::Rage),
            "threat" if c.confidence >= threshold => Some(FlagType::Threat),
            "harassment" if c.confidence >= threshold => Some(FlagType::Harassment),
            _ => None,
        };

        if let Some(flag_type) = flag {
            detected.push((flag_type, c.confidence));
        }
    }

    if detected.is_empty() {
        return None;
    }

    let mut ia_score = 0.0;
    let mut triggered: Vec<String> = Vec::new();

    for (flag_type, confidence) in &detected {
        let rule = rules
            .iter()
            .find(|r| r.flag_type == *flag_type && r.enabled);
        let base_weight = match rule {
            Some(r) => r.weight,
            None => scoring_config.weight_for(flag_type),
        };
        let weighted = base_weight * (*confidence as f64);
        ia_score += weighted;
        triggered.push(format!(
            "{}({:.0}%)",
            flag_type.as_str(),
            confidence * 100.0
        ));
    }

    let reason = format!("IA sentiment : {}", triggered.join(", "));
    Some((
        ia_score,
        detected.into_iter().map(|(f, _)| f).collect(),
        reason,
    ))
}

/// Transforme la réponse DeepSeek en signal de modération pondéré.
///
/// DeepSeek retourne une confiance de toxicité entre 0 et 1. Cette confiance
/// doit être multipliée par le poids de la règle correspondante, exactement
/// comme les classifications ONNX locales ; l'ajouter directement au score
/// rendrait les seuils configurés (2, 4, 6, 9…) inatteignables.
pub fn score_deepseek_analysis(
    analysis: &crate::ports::outbound::ai::deepseek_moderation_service::DeepSeekModerationAnalysis,
    rules: &[crate::domain::entities::system::rule::Rule],
    threshold: f32,
    scoring_config: &ScoringConfig,
) -> Option<(f64, Vec<FlagType>, String)> {
    if analysis.toxicity_score < threshold as f64 {
        return None;
    }

    let flag_for_label = |label: &str| match label.trim().to_ascii_lowercase().as_str() {
        "anger" | "angry" | "colere" | "colère" => Some(FlagType::Anger),
        "rage" | "aggressive" | "agressif" | "agression" => Some(FlagType::Rage),
        "threat" | "threatening" | "menace" => Some(FlagType::Threat),
        "harassment" | "hate" | "hate_speech" | "toxic" | "toxicity" | "harcelement"
        | "harcèlement" => Some(FlagType::Harassment),
        "insult" | "insulte" => Some(FlagType::Insult),
        "profanity" | "profanite" | "profanité" => Some(FlagType::Profanity),
        "spam" => Some(FlagType::Spam),
        "nsfw" => Some(FlagType::Nsfw),
        _ => None,
    };

    let mut detected = Vec::new();
    if let Some(flag) = flag_for_label(&analysis.sentiment) {
        detected.push(flag);
    }
    for label in &analysis.flags {
        if let Some(flag) = flag_for_label(label) {
            if !detected.contains(&flag) {
                detected.push(flag);
            }
        }
    }
    // Une réponse IA explicitement toxique doit toujours produire un poids,
    // même si le fournisseur a utilisé un libellé non encore connu.
    if detected.is_empty() {
        detected.push(FlagType::Harassment);
    }

    let confidence = analysis.toxicity_score.clamp(0.0, 1.0);
    let score = detected
        .iter()
        .map(|flag| {
            rules
                .iter()
                .find(|rule| rule.flag_type == *flag && rule.enabled)
                .map(|rule| rule.weight)
                .unwrap_or_else(|| scoring_config.weight_for(flag))
                * confidence
        })
        .sum();
    let labels = detected
        .iter()
        .map(FlagType::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let reason = format!(
        "DeepSeek [{} — {:.0}%] : {}",
        labels,
        confidence * 100.0,
        analysis.reason
    );

    Some((score, detected, reason))
}

/// Construit un contenu enrichi avec le contexte conversationnel pour l'inference IA.
/// Le message analyse est place en premier (safe si le tokenizer tronque la fin).
/// - "natural" : conversation brute separee par des retours a la ligne
/// - "tagged"  : balises [message]/[context] pour structurer l'input
fn build_contextual_content(
    content: &str,
    context: &[crate::ports::inbound::ai::analyze_message::ContextMessageEntry],
    format: &str,
) -> String {
    if context.is_empty() {
        return content.to_string();
    }
    let ctx_str: String = context
        .iter()
        .map(|m| format!("{}: {}", m.username, m.content))
        .collect::<Vec<_>>()
        .join("\n");
    match format {
        "tagged" => format!(
            "[message] {} [/message] [context] {} [/context]",
            content, ctx_str
        ),
        _ => format!("{}\n---\n{}", ctx_str, content),
    }
}

/// C5 — empêche qu'une détection IA fasse, à elle seule, basculer un message en
/// Ban AUTOMATIQUE. Le score combiné `bot + IA` peut, sur un premier message,
/// dépasser le seuil de ban sans aucune escalade. Si l'action calculée est Ban
/// alors que le score BOT seul n'atteignait pas le seuil de ban, on plafonne
/// l'action à Mute (le Ban reste atteignable via l'escalade de strikes ou une
/// décision humaine sur la carte de review). Le Ban auto déclenché par le seul
/// score bot (≥ seuil) est préservé (comportement historique).
pub(crate) fn cap_ia_induced_ban(
    action: Action,
    duration: Option<u64>,
    bot_score: f64,
    t_ban: f64,
    mute_duration_secs: u64,
) -> (Action, Option<u64>) {
    if matches!(action, Action::Ban) && bot_score < t_ban {
        (Action::Mute, Some(mute_duration_secs))
    } else {
        (action, duration)
    }
}

// `resolve_thresholds` est désormais la fonction canonique du `ScoringService`
// (résolution per-flag-type). On la réexporte pour les tests de ce module.
use crate::domain::services::moderation::scoring_service::resolve_thresholds;

#[cfg(test)]
#[path = "tests/analyze_message_service.rs"]
mod tests;
