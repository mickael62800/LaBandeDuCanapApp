//! Mode nuit automod : fenêtre horaire (gère le passage par minuit) et table
//! de dégradation des seuils de détection (divisés par ~2 avec planchers).

use super::DetectorConfig;

/// Vrai si `hour` (0-23, UTC) est dans la plage de nuit `[start, end)`.
/// `start > end` = passage par minuit (ex: 22h-8h).
pub fn is_night_hour(hour: u8, start: u8, end: u8) -> bool {
    if start > end {
        hour >= start || hour < end
    } else {
        hour >= start && hour < end
    }
}

/// Reduit les seuils de detection pour le mode nuit (seuils divises par ~2).
pub fn apply_night_mode(config: &mut DetectorConfig) {
    config.spam_repeat_char_threshold = (config.spam_repeat_char_threshold / 2).max(4);
    config.spam_repeat_word_threshold = (config.spam_repeat_word_threshold / 2).max(3);
    config.caps_threshold_chars = (config.caps_threshold_chars / 2).max(6);
    config.emoji_spam_max = (config.emoji_spam_max / 2).max(5);
    config.mentions_max = (config.mentions_max / 2).max(3);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_window_same_day() {
        // 8h-18h : fenêtre simple.
        assert!(is_night_hour(8, 8, 18));
        assert!(is_night_hour(17, 8, 18));
        assert!(!is_night_hour(18, 8, 18));
        assert!(!is_night_hour(7, 8, 18));
    }

    #[test]
    fn night_window_crossing_midnight() {
        // 22h-8h : passage par minuit.
        assert!(is_night_hour(22, 22, 8));
        assert!(is_night_hour(23, 22, 8));
        assert!(is_night_hour(0, 22, 8));
        assert!(is_night_hour(7, 22, 8));
        assert!(!is_night_hour(8, 22, 8));
        assert!(!is_night_hour(12, 22, 8));
    }

    #[test]
    fn night_mode_halves_with_floors() {
        let mut cfg = DetectorConfig {
            spam_repeat_char_threshold: 10,
            spam_repeat_word_threshold: 8,
            caps_threshold_chars: 20,
            emoji_spam_max: 12,
            mentions_max: 9,
            ..Default::default()
        };
        apply_night_mode(&mut cfg);
        assert_eq!(cfg.spam_repeat_char_threshold, 5);
        assert_eq!(cfg.spam_repeat_word_threshold, 4);
        assert_eq!(cfg.caps_threshold_chars, 10);
        assert_eq!(cfg.emoji_spam_max, 6);
        assert_eq!(cfg.mentions_max, 4);
    }

    #[test]
    fn night_mode_floors_hold() {
        let mut cfg = DetectorConfig {
            spam_repeat_char_threshold: 1,
            spam_repeat_word_threshold: 1,
            caps_threshold_chars: 1,
            emoji_spam_max: 1,
            mentions_max: 1,
            ..Default::default()
        };
        apply_night_mode(&mut cfg);
        assert_eq!(cfg.spam_repeat_char_threshold, 4);
        assert_eq!(cfg.spam_repeat_word_threshold, 3);
        assert_eq!(cfg.caps_threshold_chars, 6);
        assert_eq!(cfg.emoji_spam_max, 5);
        assert_eq!(cfg.mentions_max, 3);
    }
}
