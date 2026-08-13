use crate::sentinel::domain::entities::system::discord_ids::GuildId;
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

// ── Lecture typée d'un slice de config (vue `&[BotGuildConfig]`) ──
//
// Source unique pour les application services : mêmes sémantiques que
// `config_parsers` (vue HashMap), notamment `parse_bool_str` comme référence
// de vérité. Ne pas réécrire de `cfg_*` locaux dans les services.

use crate::sentinel::domain::entities::system::config_parsers::{
    parse_bool_str, parse_enabled_flag,
};

/// Valeur brute d'une clé de config, si présente.
pub fn cfg_str<'a>(entries: &'a [BotGuildConfig], key: &str) -> Option<&'a str> {
    entries
        .iter()
        .find(|e| e.config_key == key)
        .map(|e| e.config_value.as_str())
}

/// Flag booléen : présent → `parse_bool_str` ("true"/"1"/"yes", insensible à
/// la casse), absent → `default`.
pub fn cfg_bool(entries: &[BotGuildConfig], key: &str, default: bool) -> bool {
    cfg_str(entries, key).map(parse_bool_str).unwrap_or(default)
}

/// Entier i64 : clé absente ou non numérique → `default`.
pub fn cfg_i64(entries: &[BotGuildConfig], key: &str, default: i64) -> i64 {
    cfg_str(entries, key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Entier u64 : clé absente ou non numérique → `default`.
pub fn cfg_u64(entries: &[BotGuildConfig], key: &str, default: u64) -> u64 {
    cfg_str(entries, key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Flag `enabled` du module : absent = activé (comportement inclusif).
pub fn cfg_enabled(entries: &[BotGuildConfig]) -> bool {
    parse_enabled_flag(cfg_str(entries, "enabled"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, value: &str) -> BotGuildConfig {
        BotGuildConfig {
            id: Uuid::new_v4(),
            guild_id: "g".into(),
            bot_name: "test-bot".into(),
            config_key: key.into(),
            config_value: value.into(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn cfg_str_finds_key() {
        let e = vec![entry("a", "x"), entry("b", "y")];
        assert_eq!(cfg_str(&e, "b"), Some("y"));
        assert_eq!(cfg_str(&e, "z"), None);
    }

    #[test]
    fn cfg_bool_reference_semantics() {
        // "yes" et la casse mixte sont VRAIS (sémantique de référence).
        for v in ["true", "1", "yes", "True", "YES"] {
            assert!(cfg_bool(&[entry("f", v)], "f", false), "{v}");
        }
        // Toute autre valeur présente = false (pas le défaut).
        for v in ["false", "0", "no", "garbage"] {
            assert!(!cfg_bool(&[entry("f", v)], "f", true), "{v}");
        }
        // Absent = défaut.
        assert!(cfg_bool(&[], "f", true));
    }

    #[test]
    fn cfg_numeric_defaults() {
        assert_eq!(cfg_i64(&[entry("n", "42")], "n", 7), 42);
        assert_eq!(cfg_i64(&[entry("n", "abc")], "n", 7), 7);
        assert_eq!(cfg_u64(&[], "n", 9), 9);
    }

    #[test]
    fn cfg_enabled_inclusive_default() {
        assert!(!cfg_enabled(&[]));
        assert!(!cfg_enabled(&[entry("enabled", "false")]));
        assert!(cfg_enabled(&[entry("enabled", "yes")]));
    }
}
