/// Detection d'abus Unicode :
/// - Zalgo text (combining characters excessifs)
/// - Caracteres invisibles (zero-width spaces, joiners)
/// - Homoglyphes (melange scripts latin + cyrillique)

/// Detecte le texte zalgo (trop de combining characters sur un seul caractere).
pub fn detect_zalgo(content: &str, max_combining: usize) -> bool {
    let mut combining_count = 0u32;

    for c in content.chars() {
        if is_combining(c) {
            combining_count += 1;
            if combining_count as usize >= max_combining {
                return true;
            }
        } else {
            combining_count = 0;
        }
    }

    false
}

/// Detecte les caracteres invisibles excessifs.
pub fn detect_invisible(content: &str, max_invisible: usize) -> bool {
    let count = content.chars().filter(|&c| is_invisible(c)).count();
    count >= max_invisible
}

/// Detecte les homoglyphes : un melange latin + cyrillique DANS LE MEME MOT
/// (usurpation type « disc<cyrillique-o>rd »). Un message bilingue legitime
/// (mots latins ET mots cyrilliques separes, ex. « Привет guys ») n'est PAS
/// suspect — c'est le melange intra-mot qui trahit un homoglyphe.
pub fn detect_homoglyphs(content: &str) -> bool {
    content.split_whitespace().any(|word| {
        let mut has_latin = false;
        let mut has_cyrillic = false;
        for c in word.chars() {
            if is_latin(c) {
                has_latin = true;
            } else if is_cyrillic(c) {
                has_cyrillic = true;
            }
            if has_latin && has_cyrillic {
                return true;
            }
        }
        false
    })
}

/// Detection combinee : retourne true si l'un des checks passe.
pub fn detect(content: &str, max_combining: usize, max_invisible: usize) -> bool {
    detect_zalgo(content, max_combining)
        || detect_invisible(content, max_invisible)
        || detect_homoglyphs(content)
}

fn is_combining(c: char) -> bool {
    matches!(c,
        '\u{0300}'..='\u{036F}'   // Combining Diacritical Marks
        | '\u{0489}'              // Combining Cyrillic-Slavic
        | '\u{1AB0}'..='\u{1AFF}' // Combining Diacritical Marks Extended
        | '\u{1DC0}'..='\u{1DFF}' // Combining Diacritical Marks Supplement
        | '\u{20D0}'..='\u{20FF}' // Combining Diacritical Marks for Symbols
        | '\u{FE20}'..='\u{FE2F}' // Combining Half Marks
    )
}

fn is_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{200B}' // Zero Width Space
        | '\u{200C}' // Zero Width Non-Joiner
        | '\u{200D}' // Zero Width Joiner
        | '\u{2060}' // Word Joiner
        | '\u{FEFF}' // BOM / Zero Width No-Break Space
        | '\u{00AD}' // Soft Hyphen
        | '\u{034F}' // Combining Grapheme Joiner
        | '\u{061C}' // Arabic Letter Mark
        | '\u{180E}' // Mongolian Vowel Separator
    )
}

fn is_latin(c: char) -> bool {
    c.is_ascii_alphabetic()
}

fn is_cyrillic(c: char) -> bool {
    matches!(c, '\u{0400}'..='\u{04FF}' | '\u{0500}'..='\u{052F}')
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Zalgo ──

    #[test]
    fn zalgo_detected() {
        // "h" suivi de 5 combining characters
        let zalgo = "h\u{0300}\u{0301}\u{0302}\u{0303}\u{0304}ello";
        assert!(detect_zalgo(zalgo, 3));
    }

    #[test]
    fn zalgo_below_threshold() {
        let light = "h\u{0300}\u{0301}ello";
        assert!(!detect_zalgo(light, 3));
    }

    #[test]
    fn zalgo_clean_text() {
        assert!(!detect_zalgo("Hello world", 3));
    }

    #[test]
    fn zalgo_accented_text_ok() {
        // Les accents normaux n'ont qu'un combining character
        assert!(!detect_zalgo("cafe\u{0301}", 3));
    }

    #[test]
    fn zalgo_heavy() {
        let heavy = format!("a{}", "\u{0300}".repeat(10));
        assert!(detect_zalgo(&heavy, 3));
    }

    // ── Invisible ──

    #[test]
    fn invisible_detected() {
        let msg = format!("hello{}world{}!", '\u{200B}', '\u{200B}');
        assert!(detect_invisible(&msg, 2));
    }

    #[test]
    fn invisible_below_threshold() {
        let msg = format!("hello{}world", '\u{200B}');
        assert!(!detect_invisible(&msg, 2));
    }

    #[test]
    fn invisible_clean_text() {
        assert!(!detect_invisible("Normal text here", 5));
    }

    #[test]
    fn invisible_zero_width_joiner() {
        let msg = "\u{200D}\u{200D}\u{200D}\u{200D}\u{200D}";
        assert!(detect_invisible(msg, 5));
    }

    #[test]
    fn invisible_mixed_types() {
        let msg = format!(
            "a{}b{}c{}d{}e{}",
            '\u{200B}', '\u{200C}', '\u{200D}', '\u{2060}', '\u{FEFF}'
        );
        assert!(detect_invisible(msg.as_str(), 5));
    }

    // ── Homoglyphs ──

    #[test]
    fn homoglyph_latin_cyrillic_mix() {
        // "a" latin + "а" cyrillic (U+0430)
        let msg = "a\u{0430}dmin";
        assert!(detect_homoglyphs(msg));
    }

    #[test]
    fn homoglyph_pure_latin() {
        assert!(!detect_homoglyphs("Hello world"));
    }

    #[test]
    fn homoglyph_pure_cyrillic() {
        assert!(!detect_homoglyphs(
            "\u{041F}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}"
        )); // Привет
    }

    #[test]
    fn homoglyph_no_alpha() {
        assert!(!detect_homoglyphs("12345 !@#$%"));
    }

    #[test]
    fn homoglyph_classic_scam() {
        // "discord" avec le "o" en cyrillique (U+043E)
        let msg = "disc\u{043E}rd.gift/free";
        assert!(detect_homoglyphs(msg));
    }

    #[test]
    fn homoglyph_bilingual_message_ok() {
        // Russe + anglais dans des MOTS separes -> legitime, pas un homoglyphe.
        assert!(!detect_homoglyphs(
            "\u{041F}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442} guys gg"
        ));
    }

    // ── Combined detect ──

    #[test]
    fn detect_combined_zalgo() {
        let zalgo = format!("a{}", "\u{0300}".repeat(5));
        assert!(detect(&zalgo, 3, 5));
    }

    #[test]
    fn detect_combined_invisible() {
        let msg = "\u{200B}".repeat(6);
        assert!(detect(&msg, 3, 5));
    }

    #[test]
    fn detect_combined_homoglyph() {
        let msg = "a\u{0430}dmin";
        assert!(detect(msg, 3, 5));
    }

    #[test]
    fn detect_clean_message() {
        assert!(!detect("Salut tout le monde !", 3, 5));
    }

    #[test]
    fn detect_empty() {
        assert!(!detect("", 3, 5));
    }
}
