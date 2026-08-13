//! Helpers emoji : parsing d'une chaine stockee en DB et comparaison avec
//! un `ReactionType` recu de Discord.

use serenity::all::{EmojiId, ReactionType};

/// Parse une chaine emoji :
/// - `<:name:123456>` → custom
/// - `<a:name:123456>` → custom anime
/// - sinon → unicode (ex. "🎮")
///
/// Retourne None si la chaine est vide.
pub fn parse_reaction_type(raw: &str) -> Option<ReactionType> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    if let Some(custom) = parse_custom(s) {
        return Some(custom);
    }

    // Fallback : unicode. On ne tente pas de valider les codepoints,
    // Discord rejettera au besoin.
    Some(ReactionType::Unicode(s.to_string()))
}

fn parse_custom(s: &str) -> Option<ReactionType> {
    // Le décodage `<:name:id>` / `<a:name:id>` vit dans le core ; seul le
    // mapping vers le type Serenity reste ici.
    let r = platform_core::sentinel::domain::services::system::discord_naming::parse_emoji_ref(s)?;
    Some(ReactionType::Custom {
        animated: r.animated,
        id: EmojiId::new(r.id),
        name: Some(r.name),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_unicode() {
        let r = parse_reaction_type("🎮").unwrap();
        assert!(matches!(r, ReactionType::Unicode(_)));
    }

    #[test]
    fn parse_custom_static() {
        let r = parse_reaction_type("<:cool:123456789>").unwrap();
        match r {
            ReactionType::Custom { animated, id, name } => {
                assert!(!animated);
                assert_eq!(id.get(), 123456789);
                assert_eq!(name.as_deref(), Some("cool"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_custom_animated() {
        let r = parse_reaction_type("<a:wave:987654321>").unwrap();
        match r {
            ReactionType::Custom { animated, id, .. } => {
                assert!(animated);
                assert_eq!(id.get(), 987654321);
            }
            _ => panic!(),
        }
    }
}
