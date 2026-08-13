use super::*;

// ── normalize_game_name ──

#[test]
fn normalize_name_accepts_valid() {
    assert_eq!(normalize_game_name("Valorant").unwrap(), "Valorant");
}

#[test]
fn normalize_name_trims_whitespace() {
    assert_eq!(
        normalize_game_name("  League of Legends  ").unwrap(),
        "League of Legends"
    );
}

#[test]
fn normalize_name_rejects_empty() {
    assert_eq!(
        normalize_game_name("").unwrap_err(),
        "Le nom du jeu ne peut pas etre vide"
    );
}

#[test]
fn normalize_name_rejects_whitespace_only() {
    assert_eq!(
        normalize_game_name("   \t\n  ").unwrap_err(),
        "Le nom du jeu ne peut pas etre vide"
    );
}

#[test]
fn normalize_name_rejects_over_100_chars() {
    let long = "a".repeat(101);
    assert_eq!(
        normalize_game_name(&long).unwrap_err(),
        "Le nom du jeu ne peut pas depasser 100 caracteres"
    );
}

#[test]
fn normalize_name_accepts_exactly_100_chars() {
    let exact = "a".repeat(100);
    assert_eq!(normalize_game_name(&exact).unwrap().len(), 100);
}

#[test]
fn normalize_name_counts_unicode_graphemes_not_bytes() {
    // "é" = 2 octets en UTF-8 mais 1 caractere. 100 "é" = 200 octets mais OK.
    let accented = "é".repeat(100);
    assert!(normalize_game_name(&accented).is_ok());
    let too_many = "é".repeat(101);
    assert!(normalize_game_name(&too_many).is_err());
}

// ── normalize_optional_tag ──

#[test]
fn optional_tag_none_is_none() {
    assert_eq!(normalize_optional_tag(None), None);
}

#[test]
fn optional_tag_empty_becomes_none() {
    assert_eq!(normalize_optional_tag(Some("")), None);
    assert_eq!(normalize_optional_tag(Some("   ")), None);
}

#[test]
fn optional_tag_trims_and_preserves() {
    assert_eq!(
        normalize_optional_tag(Some("  fps  ")),
        Some("fps".to_string())
    );
}

// ── parse_role_color_hex ──

#[test]
fn parse_color_accepts_hash_prefix() {
    assert_eq!(parse_role_color_hex("#ff0000", 0), 0xff0000);
}

#[test]
fn parse_color_accepts_no_prefix() {
    assert_eq!(parse_role_color_hex("00ff00", 0), 0x00ff00);
}

#[test]
fn parse_color_trims_whitespace() {
    assert_eq!(parse_role_color_hex("  #123456  ", 0), 0x123456);
}

#[test]
fn parse_color_fallback_on_invalid() {
    assert_eq!(
        parse_role_color_hex("not-hex", DEFAULT_GAME_ROLE_COLOR),
        DEFAULT_GAME_ROLE_COLOR
    );
    assert_eq!(parse_role_color_hex("", 0x123), 0x123);
}

#[test]
fn parse_color_accepts_uppercase() {
    assert_eq!(parse_role_color_hex("#ABCDEF", 0), 0xabcdef);
}

#[test]
fn default_color_is_peter_river_blue() {
    assert_eq!(DEFAULT_GAME_ROLE_COLOR, 0x3498db);
}

// ── is_allowed_emoji_mime ──

#[test]
fn emoji_mime_accepts_png_jpeg_gif_webp() {
    assert!(is_allowed_emoji_mime("image/png"));
    assert!(is_allowed_emoji_mime("image/jpeg"));
    assert!(is_allowed_emoji_mime("image/jpg"));
    assert!(is_allowed_emoji_mime("image/gif"));
    assert!(is_allowed_emoji_mime("image/webp"));
}

#[test]
fn emoji_mime_rejects_others() {
    assert!(!is_allowed_emoji_mime("image/svg+xml"));
    assert!(!is_allowed_emoji_mime("image/bmp"));
    assert!(!is_allowed_emoji_mime("application/pdf"));
    assert!(!is_allowed_emoji_mime(""));
    assert!(!is_allowed_emoji_mime("IMAGE/PNG")); // case-sensitive
}

#[test]
fn emoji_max_bytes_is_256kb() {
    assert_eq!(MAX_EMOJI_IMAGE_BYTES, 256 * 1024);
}

// ── slugify_emoji_name ──

#[test]
fn slugify_lowercases_alpha() {
    assert_eq!(slugify_emoji_name("MyGame"), "mygame");
}

#[test]
fn slugify_preserves_underscore() {
    assert_eq!(slugify_emoji_name("my_game"), "my_game");
}

#[test]
fn slugify_replaces_whitespace_dash_dot_with_underscore() {
    assert_eq!(slugify_emoji_name("my game"), "my_game");
    assert_eq!(slugify_emoji_name("my-game"), "my_game");
    assert_eq!(slugify_emoji_name("my.game"), "my_game");
}

#[test]
fn slugify_collapses_consecutive_separators() {
    assert_eq!(slugify_emoji_name("my   game"), "my_game");
    assert_eq!(slugify_emoji_name("a...b"), "a_b");
}

#[test]
fn slugify_trims_leading_trailing_underscores() {
    assert_eq!(slugify_emoji_name("  name  "), "name");
    assert_eq!(slugify_emoji_name("_name_"), "name");
}

#[test]
fn slugify_strips_non_alphanum() {
    assert_eq!(slugify_emoji_name("hello!@#world"), "helloworld");
    assert_eq!(slugify_emoji_name("a✨b"), "ab");
}

#[test]
fn slugify_truncates_to_32_chars() {
    let long = "a".repeat(100);
    assert_eq!(slugify_emoji_name(&long).len(), 32);
}

#[test]
fn slugify_pads_to_min_2_chars() {
    assert_eq!(slugify_emoji_name("a"), "a_");
    assert_eq!(slugify_emoji_name(""), "__");
}

#[test]
fn slugify_digits_preserved() {
    assert_eq!(slugify_emoji_name("game123"), "game123");
}

// ── format_custom_emoji ──

#[test]
fn format_emoji_static() {
    assert_eq!(
        format_custom_emoji("smile", "12345", false),
        "<:smile:12345>"
    );
}

#[test]
fn format_emoji_animated() {
    assert_eq!(format_custom_emoji("wave", "67890", true), "<a:wave:67890>");
}
