/// Remplace les placeholders dans un message de bienvenue/depart.
///
/// Placeholders supportes :
/// - `{user}` → mention de l'utilisateur (<@id>)
/// - `{username}` → nom d'utilisateur (sans mention)
/// - `{server}` → nom du serveur
/// - `{count}` → nombre de membres
/// - `{years}` → nombre d'annees sur le serveur (anniversaire)
pub fn render(
    template: &str,
    user_id: &str,
    username: &str,
    server_name: &str,
    member_count: u64,
    years: Option<u64>,
) -> String {
    let mut result = template.to_string();
    result = result.replace("{user}", &format!("<@{}>", user_id));
    result = result.replace("{username}", username);
    result = result.replace("{server}", server_name);
    result = result.replace("{count}", &member_count.to_string());
    if let Some(y) = years {
        result = result.replace("{years}", &y.to_string());
    }
    result
}

/// Parse une couleur hex (avec ou sans #) en u32 pour les embeds Discord.
/// Fallback bleu Discord. Implémentation unique : `parse_role_color_hex`
/// (qui trim les espaces — un `" #FF0000"` est valide).
pub fn parse_color(hex: &str) -> u32 {
    crate::sentinel::domain::services::system::discord_naming::parse_role_color_hex(hex, 0x3498db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_all_placeholders() {
        let msg = render(
            "Bienvenue {user} ({username}) sur **{server}** ! Tu es le **{count}e** membre.",
            "123456",
            "Alice",
            "MonServeur",
            42,
            None,
        );
        assert!(msg.contains("<@123456>"));
        assert!(msg.contains("Alice"));
        assert!(msg.contains("MonServeur"));
        assert!(msg.contains("42"));
    }

    #[test]
    fn render_with_years() {
        let msg = render(
            "Bravo {user}, ca fait **{years} an(s)** !",
            "123",
            "Bob",
            "Serv",
            100,
            Some(3),
        );
        assert!(msg.contains("3 an(s)"));
    }

    #[test]
    fn render_no_years_placeholder_left() {
        let msg = render("Hello {user}", "1", "A", "S", 1, None);
        assert!(!msg.contains("{years}"));
    }

    #[test]
    fn render_empty_template() {
        let msg = render("", "1", "A", "S", 1, None);
        assert!(msg.is_empty());
    }

    #[test]
    fn render_no_placeholders() {
        let msg = render("Salut tout le monde !", "1", "A", "S", 1, None);
        assert_eq!(msg, "Salut tout le monde !");
    }

    #[test]
    fn parse_color_valid() {
        assert_eq!(parse_color("3498db"), 0x3498db);
        assert_eq!(parse_color("ff0000"), 0xff0000);
    }

    #[test]
    fn parse_color_with_hash() {
        assert_eq!(parse_color("#3498db"), 0x3498db);
    }

    #[test]
    fn parse_color_invalid_fallback() {
        assert_eq!(parse_color("not_hex"), 0x3498db);
    }

    #[test]
    fn parse_color_trims_whitespace() {
        // Régression : avant l'unification, " #FF0000" retombait sur le bleu.
        assert_eq!(parse_color(" #FF0000 "), 0xff0000);
    }

    #[test]
    fn render_leave_message() {
        let msg = render(
            "{username} nous a quittes. Nous sommes maintenant **{count}** membres.",
            "123",
            "Charlie",
            "Serv",
            99,
            None,
        );
        assert!(msg.contains("Charlie"));
        assert!(msg.contains("99"));
        assert!(!msg.contains("<@123>")); // username, pas mention
    }

    #[test]
    fn render_count_zero() {
        let msg = render("{count} membres", "1", "A", "S", 0, None);
        assert_eq!(msg, "0 membres");
    }
}
