use super::*;

/// Seuil de confiance par defaut (utilise si pas de config per-guild).
pub(super) const DEFAULT_TEXT_THRESHOLD: f32 = 0.5;

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
    entries: &[crate::sentinel::domain::entities::system::bot_config::BotGuildConfig],
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
    entries: &[crate::sentinel::domain::entities::system::bot_config::BotGuildConfig],
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
pub(super) struct TensionConfig {
    pub(super) enabled: bool,
    pub(super) buffer_size: usize,
    pub(super) threshold_warn: f64,
    pub(super) threshold_delete: f64,
    pub(super) threshold_mute: f64,
    pub(super) mute_duration_secs: u64,
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
pub(super) fn parse_tension_config(
    entries: &[crate::sentinel::domain::entities::system::bot_config::BotGuildConfig],
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
pub(super) fn tension_is_stronger(current: &Action, tension: TensionAction) -> bool {
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
