use super::*;
use platform_core::sentinel::domain::errors::DomainError;

fn err_msg(r: Result<(), DomainError>) -> String {
    match r.unwrap_err() {
        DomainError::ValidationError(m) => m,
        other => panic!("expected ValidationError, got {other:?}"),
    }
}

// ── validate_limit ──────────────────────────────────────

#[test]
fn limit_none_ok() {
    assert!(validate_limit(None).is_ok());
}

#[test]
fn limit_zero_and_normal_ok() {
    assert!(validate_limit(Some(0)).is_ok());
    assert!(validate_limit(Some(500)).is_ok());
    assert!(validate_limit(Some(MAX_QUERY_LIMIT)).is_ok());
}

#[test]
fn limit_negative_rejected() {
    assert!(validate_limit(Some(-1)).is_err());
}

#[test]
fn limit_above_ceiling_rejected() {
    // Defense en profondeur anti-DoS : au-dela du plafond absolu.
    assert!(validate_limit(Some(MAX_QUERY_LIMIT + 1)).is_err());
    assert!(validate_limit(Some(i64::MAX)).is_err());
}

// ── validate_discord_id ─────────────────────────────────

#[test]
fn discord_id_accepts_snowflake() {
    assert!(validate_discord_id("guild_id", "123456789012345678").is_ok());
}

#[test]
fn discord_id_rejects_empty() {
    assert!(err_msg(validate_discord_id("user_id", "")).contains("vide"));
}

#[test]
fn discord_id_rejects_too_long() {
    let long = "1".repeat(21);
    assert!(err_msg(validate_discord_id("user_id", &long)).contains("trop long"));
}

#[test]
fn discord_id_accepts_max_length() {
    let twenty = "1".repeat(20);
    assert!(validate_discord_id("user_id", &twenty).is_ok());
}

#[test]
fn discord_id_rejects_non_digits() {
    assert!(err_msg(validate_discord_id("user_id", "abc12345")).contains("numerique"));
    assert!(validate_discord_id("user_id", "123-456").is_err());
}

// ── validate_optional_discord_id ────────────────────────

#[test]
fn optional_discord_id_none_ok() {
    assert!(validate_optional_discord_id("user_id", &None).is_ok());
}

#[test]
fn optional_discord_id_empty_string_ok() {
    // Empty string is treated as "not provided"
    assert!(validate_optional_discord_id("user_id", &Some(String::new())).is_ok());
}

#[test]
fn optional_discord_id_invalid_fails() {
    assert!(validate_optional_discord_id("user_id", &Some("abc".into())).is_err());
}

#[test]
fn optional_discord_id_valid_ok() {
    assert!(validate_optional_discord_id("user_id", &Some("123456789012345678".into())).is_ok());
}

// ── validate_reason / validate_content / validate_name / validate_short / validate_title

#[test]
fn reason_accepts_empty() {
    assert!(validate_reason("").is_ok());
}

#[test]
fn reason_rejects_too_long() {
    let s = "a".repeat(2001);
    assert!(validate_reason(&s).is_err());
}

#[test]
fn reason_accepts_max() {
    let s = "a".repeat(2000);
    assert!(validate_reason(&s).is_ok());
}

#[test]
fn les_longueurs_se_comptent_en_caracteres_pas_en_octets() {
    // Chaque « é » pese 2 octets en UTF-8. En comptant les octets, une raison
    // de 2000 caracteres accentues etait refusee a mi-chemin de la limite
    // annoncee — et le message parlait de « chars ».
    let deux_mille_accents = "é".repeat(2000);
    assert_eq!(deux_mille_accents.len(), 4000); // octets
    assert!(validate_reason(&deux_mille_accents).is_ok());

    assert!(validate_reason(&"é".repeat(2001)).is_err());
}

#[test]
fn le_message_de_longueur_annonce_des_caracteres() {
    let msg = err_msg(validate_title(&"é".repeat(501)));
    assert!(msg.contains("501 caracteres"), "message inattendu : {msg}");
}

#[test]
fn content_rejects_empty() {
    assert!(err_msg(validate_content("")).contains("vide"));
}

#[test]
fn content_rejects_too_long() {
    let s = "a".repeat(4001);
    assert!(validate_content(&s).is_err());
}

#[test]
fn content_accepts_ok() {
    assert!(validate_content("hello").is_ok());
}

#[test]
fn name_accepts_empty() {
    assert!(validate_name("username", "").is_ok());
}

#[test]
fn name_rejects_too_long() {
    let s = "a".repeat(101);
    assert!(validate_name("username", &s).is_err());
}

#[test]
fn short_accepts_200_chars() {
    let s = "a".repeat(200);
    assert!(validate_short("action_type", &s).is_ok());
}

#[test]
fn short_rejects_201_chars() {
    let s = "a".repeat(201);
    assert!(validate_short("action_type", &s).is_err());
}

#[test]
fn title_rejects_empty() {
    assert!(err_msg(validate_title("")).contains("vide"));
}

#[test]
fn title_accepts_500_chars() {
    let s = "a".repeat(500);
    assert!(validate_title(&s).is_ok());
}

#[test]
fn title_rejects_501_chars() {
    let s = "a".repeat(501);
    assert!(validate_title(&s).is_err());
}

// ── validate_search ─────────────────────────────────────

#[test]
fn search_none_ok() {
    assert!(validate_search(&None).is_ok());
}

#[test]
fn search_short_ok() {
    assert!(validate_search(&Some("query".into())).is_ok());
}

#[test]
fn search_too_long_err() {
    assert!(validate_search(&Some("a".repeat(201))).is_err());
}

// ── pagination ──────────────────────────────────────────

#[test]
fn limit_positive_ok() {
    assert!(validate_limit(Some(0)).is_ok());
    assert!(validate_limit(Some(100)).is_ok());
    assert!(validate_limit(None).is_ok());
}

#[test]
fn limit_negative_err() {
    assert!(err_msg(validate_limit(Some(-1))).contains(">= 0"));
}

#[test]
fn offset_positive_ok() {
    assert!(validate_offset(Some(0)).is_ok());
    assert!(validate_offset(None).is_ok());
}

#[test]
fn offset_negative_err() {
    assert!(validate_offset(Some(-5)).is_err());
}

#[test]
fn pagination_both_ok() {
    assert!(validate_pagination(Some(10), Some(20)).is_ok());
}

#[test]
fn pagination_limit_invalid() {
    assert!(validate_pagination(Some(-1), Some(0)).is_err());
}

#[test]
fn pagination_offset_invalid() {
    assert!(validate_pagination(Some(0), Some(-1)).is_err());
}

// ── validate_moderation_action ──────────────────────────

#[test]
fn moderation_action_all_valid() {
    assert!(validate_moderation_action(
        "123456789012345678",
        "123456789012345678",
        "987654321098765432",
        "spam",
        "ban",
    )
    .is_ok());
}

#[test]
fn moderation_action_invalid_guild() {
    assert!(validate_moderation_action(
        "abc",
        "1".repeat(18).as_str(),
        "1".repeat(18).as_str(),
        "r",
        "ban"
    )
    .is_err());
}

#[test]
fn moderation_action_reason_too_long() {
    let long = "a".repeat(2001);
    let id = "1".repeat(18);
    assert!(validate_moderation_action(&id, &id, &id, &long, "ban").is_err());
}

// ── validate_guild_id_path / validate_guild_user_path ───

#[test]
fn guild_id_path_valid() {
    assert!(validate_guild_id_path("123456789012345678").is_ok());
}

#[test]
fn guild_id_path_invalid() {
    assert!(validate_guild_id_path("not-a-snowflake").is_err());
}

#[test]
fn guild_user_path_both_valid() {
    let id = "1".repeat(18);
    assert!(validate_guild_user_path(&id, &id).is_ok());
}

#[test]
fn guild_user_path_user_invalid() {
    let id = "1".repeat(18);
    assert!(validate_guild_user_path(&id, "abc").is_err());
}

// ── validate_bot_config ─────────────────────────────────

#[test]
fn bot_config_valid() {
    assert!(validate_bot_config("123456789012345678", "moderator", "threshold", "0.5").is_ok());
}

#[test]
fn bot_config_invalid_guild() {
    assert!(validate_bot_config("xxx", "b", "k", "v").is_err());
}

#[test]
fn bot_config_value_too_long() {
    let id = "1".repeat(18);
    let v = "a".repeat(4001);
    assert!(validate_bot_config(&id, "b", "k", &v).is_err());
}

#[test]
fn bot_config_key_too_long() {
    let id = "1".repeat(18);
    let k = "a".repeat(201);
    assert!(validate_bot_config(&id, "b", &k, "v").is_err());
}
