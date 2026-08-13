//! Parsing des listes d'IDs utilisateur pour les commandes de modération de
//! masse : multi-séparateurs, dé-mention `<@id>`/`<@!id>`, dédup en
//! conservant l'ordre.

use std::collections::HashSet;

pub fn parse_user_ids(input: &str) -> Vec<u64> {
    let mut seen = HashSet::new();
    input
        .split([',', ' ', '\n'])
        .filter_map(|s| {
            let trimmed = s
                .trim()
                .trim_start_matches("<@")
                .trim_start_matches('!')
                .trim_end_matches('>');
            trimmed.parse::<u64>().ok()
        })
        .filter(|id| seen.insert(*id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_space_separated() {
        let ids = parse_user_ids("123456 789012 345678");
        assert_eq!(ids, vec![123456, 789012, 345678]);
    }

    #[test]
    fn parse_comma_separated() {
        let ids = parse_user_ids("123456,789012,345678");
        assert_eq!(ids, vec![123456, 789012, 345678]);
    }

    #[test]
    fn parse_mixed_separators() {
        let ids = parse_user_ids("123456, 789012 345678");
        assert_eq!(ids, vec![123456, 789012, 345678]);
    }

    #[test]
    fn parse_mention_format() {
        let ids = parse_user_ids("<@123456> <@!789012>");
        assert_eq!(ids, vec![123456, 789012]);
    }

    #[test]
    fn parse_ignores_invalid() {
        let ids = parse_user_ids("123456 invalid 789012 abc");
        assert_eq!(ids, vec![123456, 789012]);
    }

    #[test]
    fn parse_empty() {
        assert!(parse_user_ids("").is_empty());
        assert!(parse_user_ids("   ").is_empty());
    }

    #[test]
    fn parse_single() {
        assert_eq!(parse_user_ids("123456"), vec![123456]);
    }

    #[test]
    fn parse_with_newlines() {
        let ids = parse_user_ids("123456\n789012\n345678");
        assert_eq!(ids, vec![123456, 789012, 345678]);
    }

    #[test]
    fn parse_dedup_keeps_order() {
        assert_eq!(parse_user_ids("2 1 2 3 1"), vec![2, 1, 3]);
    }
}
