use super::*;

fn sample_data() -> WelcomeConfigData {
    WelcomeConfigData {
        guild_id: "g1".into(),
        welcome_enabled: true,
        welcome_channel_id: Some("c-w".into()),
        welcome_message: "Bienvenue {user}".into(),
        welcome_embed_color: "0x57F287".into(),
        welcome_dm_enabled: false,
        welcome_dm_message: "dm".into(),
        leave_enabled: true,
        leave_channel_id: Some("c-l".into()),
        leave_message: "Bye".into(),
        rules_enabled: false,
        rules_channel_id: None,
        rules_message: "".into(),
        rules_role_id: None,
        rules_button_label: "OK".into(),
        age_check_enabled: false,
        age_minimum: 0,
        unverified_role_id: None,
        age_modal_question: String::new(),
        age_ban_message: String::new(),
        age_min: 5,
        age_max: 120,
        age_ban_days_per_year: 365,
        age_ban_log_channel_id: None,
        leave_embed_color: "e74c3c".into(),
        rules_embed_color: "5865f2".into(),
        counter_enabled: false,
        counter_channel_id: None,
        counter_format: "{count}".into(),
        voice_counter_enabled: false,
        voice_counter_channel_id: None,
        voice_counter_format: "En Vocal : {count}".into(),
        anniversary_enabled: false,
        anniversary_channel_id: None,
        anniversary_message: "".into(),
        rejoin_message: "Re-bonjour".into(),
        welcome_title: "".into(),
        welcome_image_url: "".into(),
        welcome_footer_text: "".into(),
        rejoin_title: "".into(),
        rejoin_image_url: "".into(),
        rejoin_footer_text: "".into(),
        leave_title: "".into(),
        leave_image_url: "".into(),
        leave_footer_text: "".into(),
        anniversary_title: "".into(),
        anniversary_image_url: "".into(),
        anniversary_footer_text: "".into(),
    }
}

// ── From<WelcomeConfigData> for WelcomeConfigDto ──

#[test]
fn from_data_maps_all_fields() {
    let dto: WelcomeConfigDto = sample_data().into();
    assert_eq!(dto.guild_id, "g1".into());
    assert!(dto.welcome_enabled);
    assert_eq!(dto.welcome_channel_id.as_deref(), Some("c-w"));
    assert_eq!(dto.welcome_message, "Bienvenue {user}");
    assert_eq!(dto.welcome_embed_color, "0x57F287");
    assert!(!dto.welcome_dm_enabled);
    assert_eq!(dto.welcome_dm_message, "dm");
    assert!(dto.leave_enabled);
    assert_eq!(dto.rules_button_label, "OK");
    assert_eq!(dto.rejoin_message, "Re-bonjour");
}

#[test]
fn from_data_preserves_none_optionals() {
    let mut d = sample_data();
    d.welcome_channel_id = None;
    d.leave_channel_id = None;
    d.rules_role_id = None;
    let dto: WelcomeConfigDto = d.into();
    assert!(dto.welcome_channel_id.is_none());
    assert!(dto.leave_channel_id.is_none());
    assert!(dto.rules_role_id.is_none());
}

// ── Serialize ──

#[test]
fn dto_serializes_to_json() {
    let dto: WelcomeConfigDto = sample_data().into();
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"guild_id\":\"g1\""));
    assert!(json.contains("\"welcome_enabled\":true"));
}

// ── Deserialize SaveWelcomeConfigDto ──

#[test]
fn save_dto_all_fields_optional() {
    let dto: SaveWelcomeConfigDto = serde_json::from_str("{}").unwrap();
    assert!(dto.welcome_enabled.is_none());
    assert!(dto.welcome_message.is_none());
    assert!(dto.leave_enabled.is_none());
}

#[test]
fn save_dto_partial_update() {
    let raw = r#"{"welcome_enabled":true,"welcome_message":"hello"}"#;
    let dto: SaveWelcomeConfigDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.welcome_enabled, Some(true));
    assert_eq!(dto.welcome_message.as_deref(), Some("hello"));
    assert!(dto.leave_enabled.is_none());
}

#[test]
fn save_dto_full_deserialize() {
    let raw = r##"{
        "welcome_enabled":true,"welcome_channel_id":"c1","welcome_message":"hi",
        "welcome_embed_color":"#FF0000","welcome_dm_enabled":false,"welcome_dm_message":"",
        "leave_enabled":true,"leave_channel_id":"c2","leave_message":"bye",
        "rules_enabled":false,"rules_button_label":"OK",
        "counter_enabled":true,"counter_channel_id":"c3","counter_format":"{count}",
        "anniversary_enabled":false,"anniversary_message":"",
        "rejoin_message":"welcome back"
    }"##;
    let dto: SaveWelcomeConfigDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.welcome_channel_id.as_deref(), Some("c1"));
    assert_eq!(dto.counter_format.as_deref(), Some("{count}"));
    assert_eq!(dto.rejoin_message.as_deref(), Some("welcome back"));
}
