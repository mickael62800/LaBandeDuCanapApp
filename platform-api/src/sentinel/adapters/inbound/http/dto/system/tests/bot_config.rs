use super::*;
use chrono::Utc;
use platform_core::sentinel::domain::entities::system::bot_config::BotDefinition;
use platform_core::sentinel::domain::entities::system::bot_config::BotGuildConfig;
use uuid::Uuid;

#[test]
fn from_bot_definition_preserves_fields() {
    let schema = serde_json::json!({"k": "v"});
    let d = BotDefinition {
        bot_name: "moderator".into(),
        display_name: "Moderator".into(),
        description: "desc".into(),
        config_schema: schema.clone(),
    };
    let dto = BotDefinitionDto::from(d);
    assert_eq!(dto.bot_name, "moderator");
    assert_eq!(dto.display_name, "Moderator");
    assert_eq!(dto.config_schema, schema);
}

#[test]
fn from_bot_guild_config_drops_id_and_timestamp() {
    let c = BotGuildConfig {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        bot_name: "b".into(),
        config_key: "k".into(),
        config_value: "v".into(),
        updated_at: Utc::now(),
    };
    let dto = BotGuildConfigDto::from(c);
    assert_eq!(dto.guild_id, "g".into());
    assert_eq!(dto.bot_name, "b");
    assert_eq!(dto.config_key, "k");
    assert_eq!(dto.config_value, "v");
}

#[test]
fn set_config_deserializes() {
    let dto: SetConfigDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "bot_name": "b", "config_key": "k", "config_value": "v"
    }))
    .unwrap();
    assert_eq!(dto.guild_id, "g".into());
    assert_eq!(dto.config_value, "v");
}

#[test]
fn delete_config_deserializes() {
    let dto: DeleteConfigDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "bot_name": "b", "config_key": "k"
    }))
    .unwrap();
    assert_eq!(dto.config_key, "k");
}
