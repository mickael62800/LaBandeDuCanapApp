//! Override de configuration par instance (key/value).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameServerConfig {
    pub server_id: Uuid,
    pub config_key: String,
    pub config_value: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<String>,
}

/// Validation d'une cle de configuration (exigence DB).
///
/// La cle doit commencer par une majuscule et ne contenir que des lettres,
/// des chiffres et des underscores.
///
/// Les minuscules sont acceptees APRES le premier caractere, contrairement au
/// SCREAMING_SNAKE_CASE strict d'origine. 7 Days to Die impose des noms de
/// variables en casse mixte — `SERVERCONFIG_BuildCreate`, `SERVERCONFIG_ZombieMove` —
/// et l'image les lit tels quels : les normaliser en majuscules les rendrait
/// inertes. La regle stricte rejetait donc toute creation de serveur 7DTD.
pub fn validate_config_key(key: &str) -> Result<(), String> {
    if key.is_empty() || key.len() > 64 {
        return Err("config_key invalide : 1-64 caracteres".into());
    }
    let mut chars = key.chars();
    let first = chars.next().ok_or("config_key vide")?;
    if !first.is_ascii_uppercase() {
        return Err("config_key doit commencer par une lettre majuscule".into());
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("config_key invalide : lettres, chiffres et underscores uniquement".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
