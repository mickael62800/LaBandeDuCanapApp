use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;
use uuid::Uuid;

use crate::sentinel::domain::entities::ai::image_analysis::ImageAnalysis;
use crate::sentinel::domain::entities::ai::image_analysis::ImageClassification;
use crate::sentinel::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::sentinel::domain::entities::moderation::infraction::Infraction;
use crate::sentinel::domain::enums::moderation::action::Action;
use crate::sentinel::domain::enums::moderation::flag_type::FlagType;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::domain::services::ai::inference_limiter::InferenceRateLimiter;
use crate::sentinel::ports::inbound::ai::analyze_image::AnalyzeImageCommand;
use crate::sentinel::ports::inbound::ai::analyze_image::AnalyzeImageUseCase;
use crate::sentinel::ports::outbound::ai::inference_service::InferenceService;
use crate::sentinel::ports::outbound::moderation::infraction_repository::InfractionRepository;
use crate::sentinel::ports::outbound::moderation::rule_repository::RuleRepository;
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::sentinel::ports::outbound::system::cache::CachePort;
/// Seuil de confiance par defaut (utilise si pas de config per-guild).
const DEFAULT_VISION_THRESHOLD: f32 = 0.5;

pub struct AnalyzeImageService {
    inference: Arc<dyn InferenceService>,
    rule_repo: Arc<dyn RuleRepository>,
    infraction_repo: Arc<dyn InfractionRepository>,
    cache: Arc<dyn CachePort>,
    /// Lecture des cles `vision_enabled` / `vision_threshold` depuis la
    /// config `automod-bot` (fusionnee avec l'ancien `ia_config` par la
    /// migration 146).
    bot_config_repo: Arc<dyn BotConfigRepository>,
    inference_limiter: Arc<InferenceRateLimiter>,
}

impl AnalyzeImageService {
    pub fn new(
        inference: Arc<dyn InferenceService>,
        rule_repo: Arc<dyn RuleRepository>,
        infraction_repo: Arc<dyn InfractionRepository>,
        cache: Arc<dyn CachePort>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
        inference_limiter: Arc<InferenceRateLimiter>,
    ) -> Self {
        Self {
            inference,
            rule_repo,
            infraction_repo,
            cache,
            bot_config_repo,
            inference_limiter,
        }
    }
}

/// Config vision parse depuis automod-bot.
struct VisionConfig {
    enabled: bool,
    threshold: f32,
    per_channel_threshold: std::collections::HashMap<String, f32>,
    hash_cache_enabled: bool,
    hash_cache_ttl_secs: u64,
    /// Toggle : force la suppression si une image NSFW est detectee, meme si le
    /// score baseline ne suffit pas a atteindre le seuil `delete`. La DECISION
    /// vit ici (core), plus dans le bot : ce dernier n'EXECUTE que l'action
    /// deja arbitree. Defaut `false` (miroir de l'ancien defaut cote bot).
    auto_delete_nsfw: bool,
    /// Idem pour les images illicites. Defaut `true` (miroir cote bot).
    auto_delete_illicit: bool,
}

/// Defaut TTL cache : 24h. Une image identique repostee dans la fenetre
/// reutilise le verdict precedent au lieu de relancer une inference.
const DEFAULT_HASH_CACHE_TTL_SECS: u64 = 86_400;

/// Parse les cles vision depuis la config automod-bot.
fn parse_vision_config(
    entries: &[crate::sentinel::domain::entities::system::bot_config::BotGuildConfig],
) -> VisionConfig {
    let mut cfg = VisionConfig {
        enabled: true,
        threshold: DEFAULT_VISION_THRESHOLD,
        per_channel_threshold: std::collections::HashMap::new(),
        hash_cache_enabled: true,
        hash_cache_ttl_secs: DEFAULT_HASH_CACHE_TTL_SECS,
        auto_delete_nsfw: false,
        auto_delete_illicit: true,
    };
    for e in entries {
        match e.config_key.as_str() {
            "vision_auto_delete_nsfw" => {
                let v = e.config_value.to_ascii_lowercase();
                cfg.auto_delete_nsfw = matches!(v.as_str(), "true" | "1" | "yes");
            }
            "vision_auto_delete_illicit" => {
                let v = e.config_value.to_ascii_lowercase();
                cfg.auto_delete_illicit = matches!(v.as_str(), "true" | "1" | "yes");
            }
            "vision_enabled" => {
                let v = e.config_value.to_ascii_lowercase();
                cfg.enabled = matches!(v.as_str(), "true" | "1" | "yes");
            }
            "vision_threshold" => {
                if let Ok(n) = e.config_value.parse::<f32>() {
                    cfg.threshold = n.clamp(0.0, 1.0);
                }
            }
            "vision_channel_thresholds" => {
                for part in e.config_value.split(',') {
                    let part = part.trim();
                    if let Some((cid, val)) = part.split_once(':') {
                        if let Ok(n) = val.trim().parse::<f32>() {
                            cfg.per_channel_threshold
                                .insert(cid.trim().to_string(), n.clamp(0.0, 1.0));
                        }
                    }
                }
            }
            "vision_hash_cache_enabled" => {
                let v = e.config_value.to_ascii_lowercase();
                cfg.hash_cache_enabled = matches!(v.as_str(), "true" | "1" | "yes");
            }
            "vision_hash_cache_ttl_secs" => {
                if let Ok(n) = e.config_value.parse::<u64>() {
                    cfg.hash_cache_ttl_secs = n.clamp(60, 7 * 86_400);
                }
            }
            _ => {}
        }
    }
    cfg
}

/// Hash siphash des bytes de l'image. Cle stable pour deduper les analyses
/// d'une meme image repostee. Non-crypto suffit (collision = cache miss
/// inoffensif, on relance juste l'inference).
fn image_hash(bytes: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Resultat cache d'une analyse vision (sans le champ infraction qui n'a pas
/// de sens pour une image partagee entre plusieurs guilds/users).
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedVisionResult {
    classifications: Vec<(String, f32)>,
    detected_labels: Vec<String>, // ["nsfw", "illicit"]
}

#[async_trait]
impl AnalyzeImageUseCase for AnalyzeImageService {
    async fn analyze_image(&self, cmd: AnalyzeImageCommand) -> Result<ImageAnalysis, DomainError> {
        // 0. Charger la config automod-bot (cles vision_enabled + vision_threshold,
        //    fusionnees depuis l'ancien ia_config via la migration 146).
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
                tracing::warn!(error = %e, guild_id = %cmd.guild_id, "Echec chargement config automod-bot (vision), utilisation defauts");
                vec![]
            }
        };
        let vcfg = parse_vision_config(&automod_entries);
        // Duree de mute configurable (defaut 600s). Cle `mute_duration_secs`
        // de la config automod-bot, identique au chemin texte/flood. Le clamp
        // 60s..28j est applique cote bot (cf. `apply_auto_protect`).
        let mute_duration_secs: u64 = automod_entries
            .iter()
            .find(|e| e.config_key == "mute_duration_secs")
            .and_then(|e| e.config_value.parse::<u64>().ok())
            .unwrap_or(600);
        // Modele de scoring (poids par flag + seuils) editable par serveur.
        // Meme source que le chemin texte : `parse_scoring_config`. Defaut =
        // constantes historiques -> comportement inchange tant que non
        // reconfigure.
        let scoring_config =
            crate::sentinel::application::ai::analyze_message_service::parse_scoring_config(
                &automod_entries,
            );
        // Override par salon si configure : channel_id -> threshold.
        let vision_threshold = vcfg
            .per_channel_threshold
            .get(cmd.channel_id.as_str())
            .copied()
            .unwrap_or(vcfg.threshold);

        // 1. Verifier que le modele vision est disponible et active
        if !vcfg.enabled || !self.inference.vision_available() {
            return Ok(ImageAnalysis {
                action: Action::None,
                reason: "Modele vision non disponible".to_string(),
                score: 0.0,
                duration: None,
                classifications: vec![],
            });
        }

        // 2a. Cache hit : si vision_hash_cache_enabled et qu'on a deja
        //     analyse cette image (meme bytes -> meme SHA-256), reutilise
        //     les classifications stockees. Evite de relancer l'inference
        //     sur des images repostees (memes/tendances/repost-bot).
        let img_hash = if vcfg.hash_cache_enabled {
            Some(image_hash(&cmd.image_bytes))
        } else {
            None
        };
        let cache_key = img_hash.as_ref().map(|h| format!("vision_hash:{h}"));
        let classifications = if let Some(key) = &cache_key {
            match self.cache.get_json(key).await {
                Ok(Some(json)) => match serde_json::from_str::<CachedVisionResult>(&json) {
                    Ok(cached) => {
                        tracing::debug!(hash = %img_hash.as_ref().unwrap(), "vision: cache HIT");
                        Some(cached.classifications.into_iter().map(|(label, confidence)| {
                            crate::sentinel::ports::outbound::ai::inference_service::InferenceClassification {
                                label, confidence,
                            }
                        }).collect::<Vec<_>>())
                    }
                    Err(_) => None,
                },
                _ => None,
            }
        } else {
            None
        };

        let classifications = match classifications {
            Some(c) => c,
            None => {
                // 2b. Preprocesser l'image (decode, resize, normalize)
                let image_tensor = preprocess_image(&cmd.image_bytes).map_err(|e| {
                    DomainError::Internal(format!("Erreur preprocessing image: {e}"))
                })?;

                // 3. Inference ONNX (rate limited)
                let _permit = self.inference_limiter.acquire().await?;
                let classifs = self
                    .inference
                    .classify_image(image_tensor)
                    .map_err(|e| DomainError::Internal(format!("Erreur inference: {e}")))?;

                // Persist cache (best-effort).
                if let Some(key) = &cache_key {
                    let cached = CachedVisionResult {
                        classifications: classifs
                            .iter()
                            .map(|c| (c.label.clone(), c.confidence))
                            .collect(),
                        detected_labels: vec![], // pas utilise au cache miss, on recalcule
                    };
                    if let Ok(json) = serde_json::to_string(&cached) {
                        let _ = self
                            .cache
                            .set_json(key, &json, vcfg.hash_cache_ttl_secs)
                            .await;
                    }
                }
                classifs
            }
        };

        info!(
            classifications = ?classifications.iter().map(|c| format!("{}:{:.2}", c.label, c.confidence)).collect::<Vec<_>>(),
            user = %cmd.username,
            "Resultat inference vision"
        );

        // 4. Convertir en DetectionFlags pour le scoring
        // Analyse d'IMAGE : aucun flag textuel n'a de sens ici, le score vient
        // entierement des classifications de vision.
        let flags = DetectionFlags {
            spam: false,
            insult: false,
            profanity: false,
            link: false,
            phishing: false,
        };

        let mut detected_labels = Vec::new();

        for c in &classifications {
            match c.label.as_str() {
                "nsfw" if c.confidence >= vision_threshold => {
                    detected_labels.push(FlagType::Nsfw);
                }
                "illicit" if c.confidence >= vision_threshold => {
                    detected_labels.push(FlagType::Illicit);
                }
                _ => {}
            }
        }

        if detected_labels.is_empty() {
            return Ok(ImageAnalysis {
                action: Action::None,
                reason: String::new(),
                score: 0.0,
                duration: None,
                classifications: classifications
                    .into_iter()
                    .map(|c| ImageClassification {
                        label: c.label,
                        confidence: c.confidence,
                    })
                    .collect(),
            });
        }

        // 5. Charger les regles et scorer
        let rules = match self.cache.get_rules(&cmd.guild_id).await? {
            Some(cached) => cached,
            None => {
                let from_db = self.rule_repo.find_by_guild(&cmd.guild_id).await?;
                if let Err(e) = self.cache.set_rules(&cmd.guild_id, &from_db).await {
                    tracing::warn!(error = %e, guild_id = %cmd.guild_id, "Echec cache set rules (vision)");
                }
                from_db
            }
        };

        // Calculer le score manuellement avec les flags IA
        let mut total_score = 0.0;
        let mut triggered: Vec<&str> = Vec::new();

        for flag_type in &detected_labels {
            let rule = rules
                .iter()
                .find(|r| r.flag_type == *flag_type && r.enabled);
            let weight = match rule {
                Some(r) => r.weight,
                None => scoring_config.weight_for(flag_type),
            };
            total_score += weight;
            triggered.push(flag_type.as_str());
        }

        // Seuils : baseline editable par serveur (ScoringConfig) + regles DB
        // per-flag-type prioritaires (comme le chemin texte).
        let (t_warn, t_delete, t_mute, t_ban) =
            crate::sentinel::domain::services::moderation::scoring_service::resolve_thresholds(
                &rules,
                &detected_labels,
                &scoring_config,
            );
        let (action, duration) = if total_score >= t_ban {
            (Action::Ban, None)
        } else if total_score >= t_mute {
            (Action::Mute, Some(mute_duration_secs))
        } else if total_score >= t_delete {
            (Action::Delete, None)
        } else if total_score >= t_warn {
            (Action::Warn, None)
        } else {
            (Action::None, None)
        };

        // Override NSFW / illicit (DECIDE = API) : si le toggle correspondant est
        // actif et que le label est detecte, on force la suppression meme si le
        // score baseline retombait sur None/Warn. Les actions plus severes
        // (Mute/Ban) sont preservees. Cette decision etait auparavant refaite
        // dans le bot a partir du texte de la `reason` — elle vit desormais ici,
        // le bot ne fait qu'EXECUTER l'action arbitree.
        let (action, duration) = if matches!(action, Action::None | Action::Warn)
            && ((vcfg.auto_delete_nsfw && detected_labels.contains(&FlagType::Nsfw))
                || (vcfg.auto_delete_illicit && detected_labels.contains(&FlagType::Illicit)))
        {
            (Action::Delete, None)
        } else {
            (action, duration)
        };

        let reason = format!(
            "Image detectee : {} (score: {:.1})",
            triggered.join(", "),
            total_score
        );

        // 6. Persister l'infraction
        // On utilise des flags factices pour le champ flags de Infraction
        // car le systeme actuel attend des DetectionFlags texte
        let infraction = Infraction {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            channel_id: cmd.channel_id,
            user_id: cmd.user_id.clone(),
            username: cmd.username.clone(),
            display_name: None,
            message_id: cmd.message_id,
            content: format!("[Image: {}]", cmd.filename),
            flags,
            score: total_score,
            action: action.clone(),
            reason: reason.clone(),
            duration,
            created_at: chrono::Utc::now(),
        };

        self.infraction_repo.save(&infraction).await?;

        // 7. Retourner le resultat
        Ok(ImageAnalysis {
            action,
            reason,
            score: total_score,
            duration,
            classifications: classifications
                .into_iter()
                .map(|c| ImageClassification {
                    label: c.label,
                    confidence: c.confidence,
                })
                .collect(),
        })
    }
}

/// Preprocesse une image brute en tensor (1, 3, 224, 224) normalise ImageNet.
fn preprocess_image(bytes: &[u8]) -> Result<ndarray::Array4<f32>, String> {
    use image::GenericImageView;

    let img = image::load_from_memory(bytes).map_err(|e| format!("Image invalide: {e}"))?;

    let resized = img.resize_exact(224, 224, image::imageops::FilterType::Triangle);

    let mut tensor = ndarray::Array4::<f32>::zeros((1, 3, 224, 224));

    // Normalisation ImageNet : mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]
    let mean = [0.485f32, 0.456, 0.406];
    let std = [0.229f32, 0.224, 0.225];

    for (x, y, pixel) in resized.pixels() {
        let rgb = pixel.0;
        for c in 0..3 {
            tensor[[0, c, y as usize, x as usize]] = (rgb[c] as f32 / 255.0 - mean[c]) / std[c];
        }
    }

    Ok(tensor)
}



#[cfg(test)]
#[path = "tests/analyze_image.rs"]
mod tests;
