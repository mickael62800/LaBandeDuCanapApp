/// Detection de spam par contenu :
/// - Repetition excessive de caracteres (aaaaaaa)
/// - Repetition de mots (buy buy buy buy buy)
/// Note : le flood (messages rapides) est gere dans le handler, pas ici.
pub fn detect(content: &str, char_threshold: usize, word_threshold: usize) -> bool {
    let trimmed = content.trim();

    if trimmed.len() < 2 {
        return false;
    }

    // Repetition de caracteres (ex: "aaaaaaa", "!!!!!!")
    let chars: Vec<char> = trimmed.chars().collect();
    let mut repeat_count = 1;
    for i in 1..chars.len() {
        if chars[i] == chars[i - 1] {
            repeat_count += 1;
            if repeat_count >= char_threshold {
                return true;
            }
        } else {
            repeat_count = 1;
        }
    }

    // Repetition de mots (ex: "buy buy buy buy buy")
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() >= word_threshold {
        let first = words[0].to_lowercase();
        if words.iter().all(|w| w.to_lowercase() == first) {
            return true;
        }
    }

    false
}

/// Detection de spam d'emojis (emojis Unicode + custom emojis Discord).
pub fn detect_emoji_spam(content: &str, threshold: usize) -> bool {
    use regex::Regex;
    use std::sync::LazyLock;

    static CUSTOM_EMOJI: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<a?:[a-zA-Z0-9_]+:\d+>").expect("regex emoji invalide"));

    let unicode_count = content
        .chars()
        .filter(|&c| {
            matches!(c,
                '\u{1F300}'..='\u{1FAFF}'
                | '\u{2600}'..='\u{27BF}'
                | '\u{FE00}'..='\u{FE0F}'
            )
        })
        .count();

    let custom_count = CUSTOM_EMOJI.find_iter(content).count();

    unicode_count + custom_count >= threshold
}

/// Detection de mentions excessives (<@!?id>, @everyone, @here).
pub fn detect_mentions(content: &str, threshold: usize) -> bool {
    use regex::Regex;
    use std::sync::LazyLock;

    static USER_MENTION: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<@!?\d+>").expect("regex mention invalide"));

    let user_count = USER_MENTION.find_iter(content).count();
    let everyone = if content.contains("@everyone") { 1 } else { 0 };
    let here = if content.contains("@here") { 1 } else { 0 };

    user_count + everyone + here >= threshold
}

/// Detection de message tout en majuscules (>= min_chars alphabetiques).
/// Ce n'est pas du spam, juste un avertissement.
pub fn detect_caps(content: &str, min_chars: usize) -> bool {
    let trimmed = content.trim();
    trimmed.len() >= min_chars
        && trimmed == trimmed.to_uppercase()
        && trimmed.chars().any(|c| c.is_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Repetition de caracteres ──

    #[test]
    fn char_repeat_6_triggers() {
        assert!(detect("aaaaaa", 6, 5));
    }

    #[test]
    fn char_repeat_5_does_not_trigger() {
        assert!(!detect("aaaaa", 6, 5));
    }

    #[test]
    fn char_repeat_in_middle_of_text() {
        assert!(detect("hello!!!!!!world", 6, 5));
        assert!(detect("salut aaaaaaa toi", 6, 5));
    }

    #[test]
    fn char_repeat_question_marks() {
        assert!(detect("quoi ??????", 6, 5));
    }

    #[test]
    fn char_repeat_mixed_no_trigger() {
        assert!(!detect("abcabc", 6, 5));
        assert!(!detect("aabbcc", 6, 5));
    }

    #[test]
    fn char_repeat_emoji_like() {
        assert!(!detect("haha", 6, 5));
        assert!(!detect("lolol", 6, 5));
    }

    #[test]
    fn char_repeat_custom_threshold() {
        assert!(detect("aaaa", 4, 5));
        assert!(!detect("aaaa", 5, 5));
    }

    // ── Repetition de mots ──

    #[test]
    fn word_repeat_5_triggers() {
        assert!(detect("buy buy buy buy buy", 6, 5));
    }

    #[test]
    fn word_repeat_4_does_not_trigger() {
        assert!(!detect("buy buy buy buy", 6, 5));
    }

    #[test]
    fn word_repeat_case_insensitive() {
        assert!(detect("SPAM Spam spam SpAm sPAM", 6, 5));
    }

    #[test]
    fn word_repeat_different_words_no_trigger() {
        assert!(!detect("buy sell trade hold wait", 6, 5));
    }

    #[test]
    fn word_repeat_with_same_punctuation_triggers() {
        assert!(detect("ok! ok! ok! ok! ok!", 6, 5));
    }

    #[test]
    fn word_repeat_mixed_punctuation_no_trigger() {
        assert!(!detect("ok! ok? ok. ok, ok;", 6, 5));
    }

    #[test]
    fn word_repeat_custom_threshold() {
        assert!(detect("lol lol lol", 6, 3));
        assert!(!detect("lol lol lol", 6, 4));
    }

    // ── Caps detection ──

    #[test]
    fn caps_long_message_triggers() {
        assert!(detect_caps("ACHETE MON PRODUIT MAINTENANT", 8));
    }

    #[test]
    fn caps_short_message_no_trigger() {
        assert!(!detect_caps("SALUT", 8));
        assert!(!detect_caps("OK COOL", 8));
    }

    #[test]
    fn caps_exactly_8_chars_triggers() {
        assert!(detect_caps("ABCDEFGH", 8));
    }

    #[test]
    fn caps_7_chars_no_trigger() {
        assert!(!detect_caps("ABCDEFG", 8));
    }

    #[test]
    fn caps_numbers_only_no_trigger() {
        assert!(!detect_caps("12345678", 8));
    }

    #[test]
    fn caps_mixed_case_no_trigger() {
        assert!(!detect_caps("Salut Comment Ca Va", 8));
    }

    #[test]
    fn caps_symbols_only_no_trigger() {
        assert!(!detect_caps("!!!!!!!!!!!", 8));
    }

    #[test]
    fn caps_with_numbers_triggers() {
        assert!(detect_caps("ALERTE 123 URGENT", 8));
    }

    #[test]
    fn caps_custom_threshold() {
        assert!(detect_caps("ABCDE", 5));
        assert!(!detect_caps("ABCDE", 6));
    }

    // ── Emoji spam ──

    #[test]
    fn emoji_spam_unicode_triggers() {
        assert!(detect_emoji_spam("🔥🔥🔥🔥🔥🔥🔥🔥🔥🔥", 10));
    }
    #[test]
    fn emoji_spam_just_below_threshold() {
        assert!(!detect_emoji_spam("🔥🔥🔥🔥🔥🔥🔥🔥🔥", 10));
    }
    #[test]
    fn emoji_spam_custom_discord_emojis() {
        let msg = "<:test:123456789> <:test:123456789> <:test:123456789> <:test:123456789> <:test:123456789>";
        assert!(detect_emoji_spam(msg, 5));
    }
    #[test]
    fn emoji_spam_animated_custom_emojis() {
        let msg = "<a:wave:111> <a:wave:111> <a:wave:111> <a:wave:111> <a:wave:111>";
        assert!(detect_emoji_spam(msg, 5));
    }
    #[test]
    fn emoji_spam_mixed_unicode_and_custom() {
        let msg = "🔥🔥🔥 <:test:123> <:test:456> <:test:789>";
        assert!(detect_emoji_spam(msg, 6));
    }
    #[test]
    fn emoji_spam_clean_text_no_trigger() {
        assert!(!detect_emoji_spam("Salut tout le monde", 10));
    }
    #[test]
    fn emoji_spam_few_emojis_no_trigger() {
        assert!(!detect_emoji_spam("super game 🔥 vraiment", 10));
    }

    // ── Mentions excessives ──

    #[test]
    fn mentions_user_ids_triggers() {
        let msg = "<@111> <@222> <@333> <@444> <@555>";
        assert!(detect_mentions(msg, 5));
    }
    #[test]
    fn mentions_just_below_threshold() {
        let msg = "<@111> <@222> <@333> <@444>";
        assert!(!detect_mentions(msg, 5));
    }
    #[test]
    fn mentions_nickname_format() {
        let msg = "<@!111> <@!222> <@!333> <@!444> <@!555>";
        assert!(detect_mentions(msg, 5));
    }
    #[test]
    fn mentions_everyone_counts() {
        let msg = "<@111> <@222> <@333> <@444> @everyone";
        assert!(detect_mentions(msg, 5));
    }
    #[test]
    fn mentions_here_counts() {
        let msg = "<@111> <@222> <@333> <@444> @here";
        assert!(detect_mentions(msg, 5));
    }
    #[test]
    fn mentions_clean_text_no_trigger() {
        assert!(!detect_mentions("Salut tout le monde", 5));
    }
    #[test]
    fn mentions_single_mention_no_trigger() {
        assert!(!detect_mentions("Hey <@123456789> comment tu vas ?", 5));
    }

    // ── Messages normaux ──

    #[test]
    fn normal_french_message() {
        assert!(!detect("Salut, comment ca va ?", 6, 5));
        assert!(!detect_caps("Salut, comment ca va ?", 8));
    }

    #[test]
    fn empty_message() {
        assert!(!detect("", 6, 5));
        assert!(!detect_caps("", 8));
    }

    #[test]
    fn single_char() {
        assert!(!detect("a", 6, 5));
        assert!(!detect_caps("A", 8));
    }

    #[test]
    fn whitespace_only() {
        assert!(!detect("   ", 6, 5));
        assert!(!detect_caps("   ", 8));
    }

    #[test]
    fn normal_long_message() {
        assert!(!detect("Bonjour tout le monde, je suis nouveau sur le serveur et je cherche des gens pour jouer", 6, 5));
    }
}
