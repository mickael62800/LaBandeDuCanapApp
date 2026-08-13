//! Prefixes de pseudo du module progression.
//!
//! Un pseudo peut porter DEUX prefixes en tete : un emoji optionnel selon le
//! role staff le plus eleve du membre, puis le prefixe de niveau `[NN]`, ex.
//! `👑[12]Alice`. Les deux parties sont recomputees ensemble a chaque
//! application pour se preserver mutuellement. Logique pure — l'adaptateur
//! (le bot) fournit les roles, la config et applique le rename Discord.

/// Limite Discord pour les nicknames de membres.
pub const DISCORD_NICKNAME_MAX: usize = 32;

/// Retire un eventuel prefixe `[NN]` en debut de chaine.
/// `[12]Darkponey` -> `Darkponey`, `Darkponey` -> `Darkponey`.
pub fn strip_level_prefix(name: &str) -> &str {
    let bytes = name.as_bytes();
    if bytes.first() != Some(&b'[') {
        return name;
    }
    let mut i = 1;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Au moins 1 chiffre + le crochet fermant.
    if i == 1 || i >= bytes.len() || bytes[i] != b']' {
        return name;
    }
    &name[i + 1..]
}

/// Extrait le niveau d'un prefixe `[NN]` en tete, si present.
/// `[12]Alice` -> `Some(12)`, `Alice` -> `None`.
pub fn parse_level_prefix(name: &str) -> Option<i32> {
    let bytes = name.as_bytes();
    if bytes.first() != Some(&b'[') {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 1 || i >= bytes.len() || bytes[i] != b']' {
        return None;
    }
    name[1..i].parse().ok()
}

/// Parse tolerant de la config `staff_role_emojis` au format
/// `role_id:emoji,role_id:emoji`. Les entrees malformees sont ignorees.
/// Emojis unicode uniquement (on coupe au premier `:`).
pub fn parse_role_emojis(csv: &str) -> Vec<(u64, String)> {
    csv.split(',')
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }
            let (id, emoji) = entry.split_once(':')?;
            let id = id.trim().parse::<u64>().ok()?;
            let emoji = emoji.trim();
            if emoji.is_empty() {
                return None;
            }
            Some((id, emoji.to_string()))
        })
        .collect()
}

/// Selectionne l'emoji du role staff le plus eleve du membre.
///
/// `member_roles` : (role_id, position) des roles du membre.
/// `mappings` : table role_id -> emoji issue de `parse_role_emojis`.
/// Retourne l'emoji du role mappe ayant la plus grande `position`, ou `None`
/// si aucun role du membre n'est mappe.
pub fn pick_emoji(member_roles: &[(u64, i64)], mappings: &[(u64, String)]) -> Option<String> {
    let mut best: Option<(i64, &str)> = None;
    for (rid, pos) in member_roles {
        if let Some((_, emoji)) = mappings.iter().find(|(mid, _)| mid == rid) {
            if best.map(|(bp, _)| *pos > bp).unwrap_or(true) {
                best = Some((*pos, emoji.as_str()));
            }
        }
    }
    best.map(|(_, e)| e.to_string())
}

/// Vrai si `c` appartient a un bloc unicode d'emoji / symbole pictographique.
///
/// Volontairement large sur les blocs emoji (symboles, pictogrammes, dingbats,
/// selecteurs de variation, ZWJ) mais EXCLUT les lettres latines accentuees
/// (ex. « É », « Ö ») pour ne pas amputer un pseudo qui commence par un
/// caractere accentue. Sert au strip generique d'un emoji staff obsolete
/// quand la config a change / a ete desactivee (cf. `strip_leading_emoji`).
fn is_emoji_scalar(c: char) -> bool {
    let u = c as u32;
    matches!(
        u,
        0x200D                    // Zero Width Joiner (sequences emoji)
        | 0x20D0..=0x20FF         // Combining Diacritical Marks for Symbols
        | 0x2190..=0x21FF         // Arrows
        | 0x2300..=0x23FF         // Miscellaneous Technical (⌚ ⏰ …)
        | 0x25A0..=0x25FF         // Geometric Shapes
        | 0x2600..=0x26FF         // Miscellaneous Symbols (⚔ …)
        | 0x2700..=0x27BF         // Dingbats
        | 0x2900..=0x297F         // Supplemental Arrows-B
        | 0x2B00..=0x2BFF         // Miscellaneous Symbols and Arrows
        | 0xFE00..=0xFE0F         // Variation Selectors
        | 0x1F000..=0x1FAFF       // Emoji supplementaires (crown, shield, flags…)
    )
}

/// Retire UN cluster emoji en tete (chaine contigue de scalaires emoji, ZWJ et
/// selecteurs de variation compris), suivi d'un espace de separation optionnel.
///
/// Generique : ne depend PAS de la config `known_emojis` courante, ce qui
/// permet de nettoyer un ancien prefixe emoji quand l'admin desactive
/// `staff_prefix_enabled` ou change `staff_role_emojis`. Un pseudo commencant
/// par une lettre normale (meme accentuee) est laisse intact.
pub fn strip_leading_emoji(name: &str) -> &str {
    let mut end = 0;
    let mut found = false;
    for (i, c) in name.char_indices() {
        if is_emoji_scalar(c) {
            found = true;
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if !found {
        return name;
    }
    let rest = &name[end..];
    rest.strip_prefix(' ').unwrap_or(rest)
}

/// Retire les prefixes connus (emoji staff puis niveau `[NN]`) pour retrouver
/// le nom de base. Robuste a l'ordre pour rester idempotent sur re-application.
/// L'emoji est matche comme prefixe exact (gere le multi-codepoint) ; un
/// espace de separation eventuel est aussi retire.
///
/// Si aucun emoji connu ne matche (config changee/desactivee), on tente un
/// strip generique d'UN cluster emoji en tete afin de nettoyer un ancien
/// prefixe qui n'est plus dans la map courante (cf. `strip_leading_emoji`).
pub fn strip_all_prefixes<'a>(name: &'a str, known_emojis: &[&str]) -> &'a str {
    let mut s = name;
    let mut matched = false;
    for e in known_emojis {
        if e.is_empty() {
            continue;
        }
        if let Some(rest) = s.strip_prefix(e) {
            s = rest.strip_prefix(' ').unwrap_or(rest);
            matched = true;
            break;
        }
    }
    if !matched {
        s = strip_leading_emoji(s);
    }
    strip_level_prefix(s)
}

/// Compose `{emoji}{[level]}{base}` en tronquant `base` (par caracteres) pour
/// respecter la limite Discord de 32. Les prefixes sont prioritaires : si
/// emoji+`[level]` atteint deja >= 32, `base` est vide.
pub fn build_nickname_full(base: &str, level: Option<i32>, emoji: Option<&str>) -> String {
    let mut prefix = String::new();
    if let Some(e) = emoji {
        prefix.push_str(e);
    }
    if let Some(l) = level {
        prefix.push_str(&format!("[{l}]"));
    }
    let prefix_len = prefix.chars().count();
    let max_base = DISCORD_NICKNAME_MAX.saturating_sub(prefix_len);
    let base_truncated: String = base.chars().take(max_base).collect();
    format!("{prefix}{base_truncated}")
}

/// Construit `[level]base` en tronquant `base` pour respecter la limite
/// Discord de 32 caracteres. (Cas historique sans emoji, conserve pour
/// compatibilite ; l'apply passe desormais par `build_nickname_full`.)
pub fn build_nickname(base: &str, level: i32) -> String {
    build_nickname_full(base, Some(level), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_no_prefix() {
        assert_eq!(strip_level_prefix("Darkponey"), "Darkponey");
    }

    #[test]
    fn strip_single_digit() {
        assert_eq!(strip_level_prefix("[1]Darkponey"), "Darkponey");
    }

    #[test]
    fn strip_multi_digit() {
        assert_eq!(strip_level_prefix("[123]Darkponey"), "Darkponey");
    }

    #[test]
    fn strip_empty_brackets_kept() {
        // [] sans chiffre n'est pas un prefixe de niveau, on ne touche pas.
        assert_eq!(strip_level_prefix("[]Darkponey"), "[]Darkponey");
    }

    #[test]
    fn strip_bracket_without_close_kept() {
        assert_eq!(strip_level_prefix("[12Darkponey"), "[12Darkponey");
    }

    #[test]
    fn strip_only_prefix() {
        assert_eq!(strip_level_prefix("[5]"), "");
    }

    #[test]
    fn build_short_name() {
        assert_eq!(build_nickname("Alice", 12), "[12]Alice");
    }

    #[test]
    fn build_truncates_long_base() {
        let base = "a".repeat(40);
        let n = build_nickname(&base, 5);
        assert_eq!(n.chars().count(), DISCORD_NICKNAME_MAX);
        assert!(n.starts_with("[5]"));
    }

    #[test]
    fn build_large_level() {
        let n = build_nickname("Bob", 999);
        assert_eq!(n, "[999]Bob");
    }

    // ── parse_level_prefix ──

    #[test]
    fn parse_level_ok() {
        assert_eq!(parse_level_prefix("[12]Alice"), Some(12));
        assert_eq!(parse_level_prefix("[7]"), Some(7));
    }

    #[test]
    fn parse_level_none() {
        assert_eq!(parse_level_prefix("Alice"), None);
        assert_eq!(parse_level_prefix("[]Alice"), None);
        assert_eq!(parse_level_prefix("[12Alice"), None);
    }

    // ── parse_role_emojis ──

    #[test]
    fn parse_emojis_basic() {
        let m = parse_role_emojis("111:\u{1f451},222:\u{1f6e1}\u{fe0f},333:\u{2694}\u{fe0f}");
        assert_eq!(
            m,
            vec![
                (111, "\u{1f451}".to_string()),
                (222, "\u{1f6e1}\u{fe0f}".to_string()),
                (333, "\u{2694}\u{fe0f}".to_string()),
            ]
        );
    }

    #[test]
    fn parse_emojis_skips_malformed() {
        // entree sans `:`, id non numerique, emoji vide, espaces -> ignorees.
        let m = parse_role_emojis("nope, 111 : \u{1f451} ,abc:x,222:,333:\u{2694}");
        assert_eq!(
            m,
            vec![
                (111, "\u{1f451}".to_string()),
                (333, "\u{2694}".to_string())
            ]
        );
    }

    #[test]
    fn parse_emojis_empty() {
        assert!(parse_role_emojis("").is_empty());
    }

    // ── pick_emoji : priorite au role le plus haut ──

    #[test]
    fn pick_highest_role_wins() {
        let mappings = vec![
            (111, "\u{1f451}".to_string()), // pos 30
            (222, "\u{1f6e1}".to_string()), // pos 10
        ];
        let roles = vec![(222, 10i64), (111, 30i64), (999, 5i64)];
        assert_eq!(pick_emoji(&roles, &mappings), Some("\u{1f451}".to_string()));
    }

    #[test]
    fn pick_none_when_no_mapped_role() {
        let mappings = vec![(111, "\u{1f451}".to_string())];
        let roles = vec![(999, 50i64)];
        assert_eq!(pick_emoji(&roles, &mappings), None);
    }

    // ── strip_all_prefixes : idempotence / ordre ──

    #[test]
    fn strip_all_emoji_then_level() {
        let known = ["\u{1f451}"];
        assert_eq!(strip_all_prefixes("\u{1f451}[12]Alice", &known), "Alice");
    }

    #[test]
    fn strip_all_emoji_with_space() {
        let known = ["\u{1f451}"];
        assert_eq!(strip_all_prefixes("\u{1f451} [12]Alice", &known), "Alice");
    }

    #[test]
    fn strip_all_multi_codepoint_emoji() {
        let known = ["\u{1f6e1}\u{fe0f}"];
        assert_eq!(strip_all_prefixes("\u{1f6e1}\u{fe0f}[3]Bob", &known), "Bob");
    }

    #[test]
    fn strip_all_no_prefix() {
        let known = ["\u{1f451}"];
        assert_eq!(strip_all_prefixes("Alice", &known), "Alice");
    }

    #[test]
    fn strip_all_level_only() {
        let known: [&str; 0] = [];
        assert_eq!(strip_all_prefixes("[42]Alice", &known), "Alice");
    }

    // ── BUG #6 : strip robuste a une config emoji desactivee/changee ──

    #[test]
    fn strip_all_emoji_not_in_known_disabled_config() {
        // Feature staff desactivee -> known vide, mais un ancien emoji reste
        // colle au pseudo. Il doit quand meme etre retire.
        let known: [&str; 0] = [];
        assert_eq!(strip_all_prefixes("\u{1f451}[12]Alice", &known), "Alice");
    }

    #[test]
    fn strip_all_emoji_changed_map() {
        // La map a change : l'emoji present n'est plus dans `known`.
        let known = ["\u{1f6e1}\u{fe0f}"];
        assert_eq!(strip_all_prefixes("\u{1f451}[7]Bob", &known), "Bob");
    }

    #[test]
    fn strip_all_multi_codepoint_emoji_not_in_known() {
        // Emoji ZWJ multi-scalaire retire generiquement (config changee).
        let known: [&str; 0] = [];
        assert_eq!(strip_all_prefixes("\u{1f6e1}\u{fe0f}[3]Bob", &known), "Bob");
    }

    #[test]
    fn strip_all_accented_base_untouched() {
        // Un pseudo commencant par une lettre accentuee n'est PAS ampute.
        let known: [&str; 0] = [];
        assert_eq!(strip_all_prefixes("\u{c9}lise", &known), "\u{c9}lise");
    }

    #[test]
    fn strip_all_midname_emoji_untouched() {
        // Emoji place par l'utilisateur EN MILIEU de nom : non leading -> intact.
        let known: [&str; 0] = [];
        assert_eq!(
            strip_all_prefixes("Ali\u{1f451}ce", &known),
            "Ali\u{1f451}ce"
        );
    }

    #[test]
    fn strip_all_generic_reapply_idempotent() {
        // Re-strip d'un nom deja nettoye ne change rien.
        let known: [&str; 0] = [];
        let once = strip_all_prefixes("\u{1f451}[12]Alice", &known);
        assert_eq!(strip_all_prefixes(once, &known), "Alice");
    }

    #[test]
    fn strip_leading_emoji_only_one_cluster() {
        // Deux clusters emoji separes par espace : seul le premier cluster +
        // l'espace sont retires (le bot n'ajoute jamais qu'un seul emoji).
        assert_eq!(
            strip_leading_emoji("\u{1f451} \u{1f6e1}\u{fe0f}Alice"),
            "\u{1f6e1}\u{fe0f}Alice"
        );
    }

    // ── build_nickname_full ──

    #[test]
    fn full_emoji_and_level() {
        assert_eq!(
            build_nickname_full("Alice", Some(12), Some("\u{1f451}")),
            "\u{1f451}[12]Alice"
        );
    }

    #[test]
    fn full_no_emoji_no_level_is_base() {
        assert_eq!(build_nickname_full("Alice", None, None), "Alice");
    }

    #[test]
    fn full_level_only_matches_legacy() {
        // byte-for-byte identique a build_nickname (comportement historique).
        assert_eq!(
            build_nickname_full("Alice", Some(5), None),
            build_nickname("Alice", 5)
        );
    }

    #[test]
    fn full_truncates_base_with_emoji() {
        let base = "a".repeat(40);
        let n = build_nickname_full(&base, Some(5), Some("\u{1f451}"));
        assert_eq!(n.chars().count(), DISCORD_NICKNAME_MAX);
        assert!(n.starts_with("\u{1f451}[5]"));
    }

    #[test]
    fn full_prefix_priority_drops_base() {
        // emoji + niveau enorme : le prefixe est conserve, base tronquee.
        let n = build_nickname_full("Alice", Some(1234567890), Some("\u{1f451}"));
        assert!(n.starts_with("\u{1f451}[1234567890]"));
    }

    #[test]
    fn full_reapply_idempotent() {
        let known = ["\u{1f451}"];
        let first = build_nickname_full("Alice", Some(12), Some("\u{1f451}"));
        let base = strip_all_prefixes(&first, &known);
        let second = build_nickname_full(base, Some(12), Some("\u{1f451}"));
        assert_eq!(first, second);
    }
}
