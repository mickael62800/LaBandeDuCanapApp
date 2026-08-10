//! Config bot par guilde (clé/valeur), lue depuis `bot_guild_config`.
//!
//! Nexus n'expose que les accesseurs qu'il consomme réellement. La sémantique
//! de référence du dépôt est `sentinel-core/…/system/config_parsers.rs` :
//! si un besoin de parsing apparaît ici (flag `enabled`, lignes `label|value`),
//! l'aligner sur ce module plutôt que d'en réinventer une variante locale.
//! Les helpers `parse_enabled_flag` / `cfg_enabled` ont été retirés d'ici :
//! personne ne les appelait, et leur défaut était l'inverse du fail-closed
//! adopté par Sentinel (clé absente = module désactivé).

use crate::domain::entities::system::discord_ids::GuildId;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotGuildConfig {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub bot_name: String,
    pub config_key: String,
    pub config_value: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotDefinition {
    pub bot_name: String,
    pub display_name: String,
    pub description: String,
    pub config_schema: serde_json::Value,
}

/// "true"/"1"/"yes" (insensible à la casse) => true, tout le reste => false.
fn parse_bool_str(v: &str) -> bool {
    matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes")
}

/// Valeur brute d'une clé de config, si présente.
pub fn cfg_str<'a>(entries: &'a [BotGuildConfig], key: &str) -> Option<&'a str> {
    entries
        .iter()
        .find(|e| e.config_key == key)
        .map(|e| e.config_value.as_str())
}

/// Flag booléen : présent => `parse_bool_str`, absent => `default`.
pub fn cfg_bool(entries: &[BotGuildConfig], key: &str, default: bool) -> bool {
    cfg_str(entries, key).map(parse_bool_str).unwrap_or(default)
}

/// Entier i64 : clé absente ou non numérique => `default`.
pub fn cfg_i64(entries: &[BotGuildConfig], key: &str, default: i64) -> i64 {
    cfg_str(entries, key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, value: &str) -> BotGuildConfig {
        BotGuildConfig {
            id: Uuid::nil(),
            guild_id: GuildId::new("guild"),
            bot_name: "game-portal".into(),
            config_key: key.into(),
            config_value: value.into(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn reads_first_matching_raw_value() {
        let entries = vec![entry("limit", "12"), entry("limit", "99")];
        assert_eq!(cfg_str(&entries, "limit"), Some("12"));
        assert_eq!(cfg_str(&entries, "missing"), None);
    }

    #[test]
    fn parses_only_explicit_true_boolean_values() {
        let entries = vec![
            entry("true_word", "TRUE"),
            entry("one", "1"),
            entry("yes", "YeS"),
            entry("false_word", "on"),
        ];
        assert!(cfg_bool(&entries, "true_word", false));
        assert!(cfg_bool(&entries, "one", false));
        assert!(cfg_bool(&entries, "yes", false));
        assert!(!cfg_bool(&entries, "false_word", true));
        assert!(cfg_bool(&entries, "missing", true));
    }

    #[test]
    fn parses_integer_or_returns_default() {
        let entries = vec![
            entry("positive", "42"),
            entry("negative", "-7"),
            entry("bad", "x"),
        ];
        assert_eq!(cfg_i64(&entries, "positive", 0), 42);
        assert_eq!(cfg_i64(&entries, "negative", 0), -7);
        assert_eq!(cfg_i64(&entries, "bad", 10), 10);
        assert_eq!(cfg_i64(&entries, "missing", 10), 10);
    }
}
