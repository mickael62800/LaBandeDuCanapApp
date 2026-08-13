//! Règles de nommage Discord. Deux slugifieurs coexistent volontairement dans
//! le repo, avec des contraintes Discord distinctes :
//! - `slugify_channel_name` (ici) : noms de SALON — séparateur `-`, 90 chars.
//! - `slugify_emoji_name` (retire avec le systeme jeux) : noms d emoji —
//!   séparateur `_`, 32 chars, minimum 2.

/// Nettoie un nom pour en faire un nom de salon Discord valide (texte).
pub fn slugify_channel_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(90)
        .collect()
}

/// Référence d'emoji custom Discord, décodée depuis `<:name:id>` /
/// `<a:name:id>`. Le mapping vers le type Serenity reste dans le bot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmojiRef {
    pub animated: bool,
    pub name: String,
    pub id: u64,
}

/// Parse une référence d'emoji custom `<:name:id>` ou `<a:name:id>`.
/// `None` si la chaîne n'est pas une référence custom valide (l'appelant
/// retombe alors sur l'unicode). Tolère un `:` dans le nom (rsplit).
pub fn parse_emoji_ref(s: &str) -> Option<EmojiRef> {
    let inner = s.trim().strip_prefix('<')?.strip_suffix('>')?;
    let (animated, rest) = if let Some(r) = inner.strip_prefix("a:") {
        (true, r)
    } else if let Some(r) = inner.strip_prefix(':') {
        (false, r)
    } else {
        return None;
    };
    let (name, id_str) = rest.rsplit_once(':')?;
    let id: u64 = id_str.parse().ok()?;
    Some(EmojiRef {
        animated,
        name: name.to_string(),
        id,
    })
}

/// Parse strict d'une couleur hex `#RRGGBB`/`RRGGBB` (6 chiffres exigés).
/// `None` si invalide — pour les configs où l'on veut distinguer « invalide »
/// de « défaut » (cf. `parse_role_color_hex` pour la variante à fallback).
pub fn parse_hex_color_strict(s: &str) -> Option<u32> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    u32::from_str_radix(h, 16).ok()
}

/// Parse laxiste d'une couleur hex `#RRGGBB`/`RRGGBB` avec fallback.
/// (Anciennement `entities::casino::game::parse_role_color_hex` — deplace ici
/// lors du retrait temporaire du systeme jeux.)
pub fn parse_role_color_hex(hex: &str, fallback: u32) -> u32 {
    u32::from_str_radix(hex.trim().trim_start_matches('#'), 16).unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_and_dashes() {
        assert_eq!(slugify_channel_name("Mon Serveur FR!"), "mon-serveur-fr");
    }

    #[test]
    fn trims_leading_trailing_dashes() {
        assert_eq!(slugify_channel_name("--abc--"), "abc");
        assert_eq!(slugify_channel_name("!!!"), "");
    }

    #[test]
    fn caps_at_90_chars() {
        let long = "a".repeat(120);
        assert_eq!(slugify_channel_name(&long).chars().count(), 90);
    }

    #[test]
    fn keeps_unicode_alphanumerics() {
        assert_eq!(slugify_channel_name("Café été"), "café-été");
    }

    #[test]
    fn emoji_ref_static_and_animated() {
        assert_eq!(
            parse_emoji_ref("<:cool:123>"),
            Some(EmojiRef {
                animated: false,
                name: "cool".into(),
                id: 123
            })
        );
        assert_eq!(
            parse_emoji_ref("<a:wave:9>"),
            Some(EmojiRef {
                animated: true,
                name: "wave".into(),
                id: 9
            })
        );
    }

    #[test]
    fn emoji_ref_name_with_colon_and_invalid() {
        // rsplit : le dernier `:` sépare l'id, le nom peut en contenir.
        assert_eq!(parse_emoji_ref("<:a:b:42>").unwrap().name, "a:b");
        assert_eq!(parse_emoji_ref("🎮"), None);
        assert_eq!(parse_emoji_ref("<:noid>"), None);
        assert_eq!(parse_emoji_ref("<:name:abc>"), None);
    }

    #[test]
    fn hex_color_strict() {
        assert_eq!(parse_hex_color_strict("#ff5e5e"), Some(0xff5e5e));
        assert_eq!(parse_hex_color_strict(" FF5E5E "), Some(0xff5e5e));
        assert_eq!(parse_hex_color_strict("#fff"), None);
        assert_eq!(parse_hex_color_strict("zzzzzz"), None);
    }
}
