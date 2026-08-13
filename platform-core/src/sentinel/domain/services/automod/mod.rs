//! Détecteurs automod — service de domaine PUR (regex + heuristiques, aucune
//! dépendance Discord/infra). Partagé par le bot (analyse locale d'un message)
//! et réutilisable par l'API/worker. Extrait de sentinel-bot (archi hexagonale).

pub mod adaptive_slowmode;
pub mod insult;
pub mod link;
pub mod night_mode;
pub mod phishing;
pub mod spam;
pub mod unicode;

/// Configuration des détecteurs, construite depuis la guild config.
#[derive(Debug)]
pub struct DetectorConfig {
    pub spam_enabled: bool,
    pub spam_repeat_char_threshold: usize,
    pub spam_repeat_word_threshold: usize,
    pub caps_enabled: bool,
    pub caps_threshold_chars: usize,
    pub insult_enabled: bool,
    pub insult_custom_words: Vec<String>,
    pub link_enabled: bool,
    pub allow_discord_invites: bool,
    pub allowed_domains: Vec<String>,
    pub phishing_enabled: bool,
    pub phishing_extra_whitelist: Vec<String>,
    pub emoji_spam_enabled: bool,
    pub emoji_spam_max: usize,
    pub mentions_enabled: bool,
    pub mentions_max: usize,
    pub suspicious_files_enabled: bool,
    pub unicode_enabled: bool,
    pub unicode_max_combining: usize,
    pub unicode_max_invisible: usize,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            spam_enabled: true,
            spam_repeat_char_threshold: 6,
            spam_repeat_word_threshold: 5,
            caps_enabled: true,
            caps_threshold_chars: 8,
            insult_enabled: true,
            insult_custom_words: vec![],
            link_enabled: true,
            allow_discord_invites: false,
            allowed_domains: vec![],
            phishing_enabled: true,
            phishing_extra_whitelist: vec![],
            emoji_spam_enabled: true,
            emoji_spam_max: 10,
            mentions_enabled: true,
            mentions_max: 5,
            suspicious_files_enabled: true,
            unicode_enabled: true,
            unicode_max_combining: 3,
            unicode_max_invisible: 5,
        }
    }
}

/// Résultat de l'analyse locale d'un message.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DetectionFlags {
    pub spam: bool,
    /// Insulte CIBLEE uniquement.
    pub insult: bool,
    /// Juron d'exclamation, sans insulte ciblee.
    pub profanity: bool,
    pub link: bool,
    pub phishing: bool,
}

/// Analyse un message et retourne les flags de détection.
/// Chaque détecteur est skippé si désactivé dans la config.
pub fn analyze(content: &str, config: &DetectorConfig) -> DetectionFlags {
    DetectionFlags {
        spam: config.spam_enabled
            && (spam::detect(
                content,
                config.spam_repeat_char_threshold,
                config.spam_repeat_word_threshold,
            ) || (config.emoji_spam_enabled
                && spam::detect_emoji_spam(content, config.emoji_spam_max))
                || (config.mentions_enabled
                    && spam::detect_mentions(content, config.mentions_max))
                || (config.unicode_enabled
                    && unicode::detect(
                        content,
                        config.unicode_max_combining,
                        config.unicode_max_invisible,
                    ))),
        insult: config.insult_enabled && insult::detect(content, &config.insult_custom_words),
        profanity: config.insult_enabled
            && insult::detect_juron(content, &config.insult_custom_words),
        link: config.link_enabled
            && link::detect(
                content,
                config.allow_discord_invites,
                &config.allowed_domains,
            ),
        phishing: config.phishing_enabled
            && phishing::detect(content, &config.phishing_extra_whitelist),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> DetectorConfig {
        DetectorConfig::default()
    }

    #[test]
    fn clean_message_no_flags() {
        let f = analyze("Salut, on fait une game ce soir ?", &default_config());
        assert!(!f.spam && !f.insult && !f.link && !f.phishing);
    }

    #[test]
    fn spam_only() {
        let f = analyze("aaaaaaa", &default_config());
        assert!(f.spam);
        assert!(!f.insult && !f.link && !f.phishing);
    }

    #[test]
    fn insult_only() {
        let f = analyze("t'es un connard", &default_config());
        assert!(f.insult);
        assert!(!f.spam && !f.phishing);
    }

    #[test]
    fn link_only() {
        let f = analyze("Va voir https://example.com", &default_config());
        assert!(f.link);
        assert!(!f.spam && !f.insult && !f.phishing);
    }

    #[test]
    fn phishing_detected() {
        let f = analyze("Free Discord Nitro click here", &default_config());
        assert!(f.phishing);
    }

    #[test]
    fn insult_with_link() {
        let f = analyze("fdp regarde https://example.com", &default_config());
        assert!(f.insult && f.link);
    }

    #[test]
    fn spam_with_insult() {
        // « merde » repete : spam + juron. Ce n'est plus une insulte ciblee,
        // c'est justement ce que la separation apporte.
        let f = analyze("merde merde merde merde merde", &default_config());
        assert!(f.spam && f.profanity && !f.insult);
    }

    #[test]
    fn spam_avec_insulte_ciblee() {
        let f = analyze("connard connard connard connard connard", &default_config());
        assert!(f.spam && f.insult);
        // Les deux flags ne se cumulent pas : le message compterait double.
        assert!(!f.profanity);
    }

    #[test]
    fn phishing_link_combo() {
        let f = analyze("https://dlscord.gift/free-nitro", &default_config());
        assert!(f.link && f.phishing);
    }

    #[test]
    fn empty_message() {
        let f = analyze("", &default_config());
        assert!(!f.spam && !f.insult && !f.link && !f.phishing);
    }

    #[test]
    fn spam_disabled_skips_detection() {
        let config = DetectorConfig {
            spam_enabled: false,
            ..DetectorConfig::default()
        };
        let f = analyze("aaaaaaa", &config);
        assert!(!f.spam);
    }

    #[test]
    fn insult_disabled_skips_detection() {
        let config = DetectorConfig {
            insult_enabled: false,
            ..DetectorConfig::default()
        };
        let f = analyze("t'es un connard", &config);
        assert!(!f.insult);
    }

    #[test]
    fn link_disabled_skips_detection() {
        let config = DetectorConfig {
            link_enabled: false,
            ..DetectorConfig::default()
        };
        let f = analyze("https://example.com", &config);
        assert!(!f.link);
    }

    #[test]
    fn phishing_disabled_skips_detection() {
        let config = DetectorConfig {
            phishing_enabled: false,
            ..DetectorConfig::default()
        };
        let f = analyze("Free Discord Nitro click here", &config);
        assert!(!f.phishing);
    }

    #[test]
    fn custom_insult_word_detected() {
        let config = DetectorConfig {
            insult_custom_words: vec!["badword".to_string()],
            ..DetectorConfig::default()
        };
        let f = analyze("tu es un badword", &config);
        assert!(f.insult);
    }

    #[test]
    fn allowed_domain_not_flagged() {
        let config = DetectorConfig {
            allowed_domains: vec!["example.com".to_string()],
            ..DetectorConfig::default()
        };
        let f = analyze("https://example.com/page", &config);
        assert!(!f.link);
    }

    #[test]
    fn emoji_spam_triggers_spam_flag() {
        let f = analyze("🔥🔥🔥🔥🔥🔥🔥🔥🔥🔥", &default_config());
        assert!(f.spam);
    }

    #[test]
    fn emoji_spam_disabled_skips() {
        let config = DetectorConfig {
            emoji_spam_enabled: false,
            ..DetectorConfig::default()
        };
        // Utilise des custom emojis Discord variés pour éviter le trigger char-repeat
        let msg = "<:a:1> <:b:2> <:c:3> <:d:4> <:e:5> <:f:6> <:g:7> <:h:8> <:i:9> <:j:10>";
        let f = analyze(msg, &config);
        assert!(!f.spam);
    }

    #[test]
    fn mentions_triggers_spam_flag() {
        let f = analyze("<@1> <@2> <@3> <@4> <@5>", &default_config());
        assert!(f.spam);
    }

    #[test]
    fn mentions_disabled_skips() {
        let config = DetectorConfig {
            mentions_enabled: false,
            ..DetectorConfig::default()
        };
        let f = analyze("<@1> <@2> <@3> <@4> <@5>", &config);
        assert!(!f.spam);
    }

    #[test]
    fn unicode_abuse_triggers_spam() {
        // Zalgo text
        let zalgo = format!("a{}", "\u{0300}".repeat(5));
        let f = analyze(&zalgo, &default_config());
        assert!(f.spam);
    }

    #[test]
    fn unicode_disabled_skips() {
        let config = DetectorConfig {
            unicode_enabled: false,
            ..DetectorConfig::default()
        };
        let zalgo = format!("a{}", "\u{0300}".repeat(5));
        let f = analyze(&zalgo, &config);
        assert!(!f.spam);
    }

    #[test]
    fn homoglyph_triggers_spam() {
        // Latin "a" + Cyrillic "а" (U+0430)
        let f = analyze("disc\u{043E}rd", &default_config());
        assert!(f.spam);
    }

    #[test]
    fn discord_invite_allowed_when_configured() {
        let config = DetectorConfig {
            allow_discord_invites: true,
            ..DetectorConfig::default()
        };
        let f = analyze("rejoins discord.gg/monserveur", &config);
        assert!(!f.link);
    }
}
