//! Configuration helpers for the automod module : couleurs d'embeds,
//! construction de `DetectorConfig`, detection mode nuit.

use crate::shared::api_client::BaseApiClient;

use super::detectors::DetectorConfig;

/// Couleurs des embeds lues depuis la config guild.
pub(super) struct EmbedColors {
    pub(super) warn: u32,
    pub(super) delete: u32,
    pub(super) mute: u32,
    pub(super) ban: u32,
}

/// Construit la config des detecteurs depuis la guild config.
pub(super) fn build_detector_config(
    config: &std::collections::HashMap<String, String>,
) -> DetectorConfig {
    DetectorConfig {
        spam_enabled: BaseApiClient::config_bool(config, "spam_detection_enabled", true),
        spam_repeat_char_threshold: BaseApiClient::config_u64(
            config,
            "spam_repeat_char_threshold",
            6,
        )
        .max(1) as usize,
        spam_repeat_word_threshold: BaseApiClient::config_u64(
            config,
            "spam_repeat_word_threshold",
            5,
        )
        .max(1) as usize,
        caps_enabled: BaseApiClient::config_bool(config, "caps_warning_enabled", true),
        caps_threshold_chars: BaseApiClient::config_u64(config, "caps_threshold_chars", 8).max(1)
            as usize,
        insult_enabled: BaseApiClient::config_bool(config, "insult_detection_enabled", true),
        insult_custom_words: crate::shared::parsers::split_csv(&BaseApiClient::config_or(
            config,
            "insult_custom_words",
            "",
        )),
        link_enabled: BaseApiClient::config_bool(config, "link_detection_enabled", true),
        allow_discord_invites: BaseApiClient::config_bool(config, "allow_discord_invites", false),
        allowed_domains: crate::shared::parsers::split_csv(&BaseApiClient::config_or(
            config,
            "allowed_domains",
            "",
        )),
        phishing_enabled: BaseApiClient::config_bool(config, "phishing_detection_enabled", true),
        phishing_extra_whitelist: crate::shared::parsers::split_csv(&BaseApiClient::config_or(
            config,
            "phishing_extra_whitelist",
            "",
        )),
        emoji_spam_enabled: BaseApiClient::config_bool(config, "emoji_spam_enabled", true),
        emoji_spam_max: BaseApiClient::config_u64(config, "emoji_spam_max", 10).max(1) as usize,
        mentions_enabled: BaseApiClient::config_bool(config, "mentions_enabled", true),
        mentions_max: BaseApiClient::config_u64(config, "mentions_max", 5).max(1) as usize,
        suspicious_files_enabled: BaseApiClient::config_bool(
            config,
            "suspicious_files_enabled",
            true,
        ),
        // `suspicious_file_extensions` n'est plus lu ici : la regle vit cote API
        // (`evaluate_attachments`), qui lit cette cle depuis la config guild.
        unicode_enabled: BaseApiClient::config_bool(config, "unicode_detection_enabled", true),
        unicode_max_combining: BaseApiClient::config_u64(config, "unicode_max_combining", 3).max(1)
            as usize,
        unicode_max_invisible: BaseApiClient::config_u64(config, "unicode_max_invisible", 5).max(1)
            as usize,
    }
}

/// Construit les couleurs d'embeds depuis la guild config.
pub(super) fn build_embed_colors(
    config: &std::collections::HashMap<String, String>,
) -> EmbedColors {
    EmbedColors {
        warn: parse_color(
            &BaseApiClient::config_or(config, "color_warn", "f59e0b"),
            0xf59e0b,
        ),
        delete: parse_color(
            &BaseApiClient::config_or(config, "color_delete", "f97316"),
            0xf97316,
        ),
        mute: parse_color(
            &BaseApiClient::config_or(config, "color_mute", "ef4444"),
            0xef4444,
        ),
        ban: parse_color(
            &BaseApiClient::config_or(config, "color_ban", "dc2626"),
            0xdc2626,
        ),
    }
}

/// Parse une couleur hex (avec ou sans #) vers u32. Retourne `default` si
/// invalide. Implémentation unique du core (trim les espaces).
use platform_core::sentinel::domain::services::system::discord_naming::parse_role_color_hex as parse_color;

/// Verifie si l'heure actuelle est dans la plage de nuit. La règle (fenêtre
/// passant minuit) vit dans le core ; le bot ne fournit que l'horloge.
pub(super) fn is_night_mode(start: u8, end: u8) -> bool {
    let hour = time::OffsetDateTime::now_utc().hour();
    platform_core::sentinel::domain::services::automod::night_mode::is_night_hour(hour, start, end)
}

pub(super) use platform_core::sentinel::domain::services::automod::night_mode::apply_night_mode;
