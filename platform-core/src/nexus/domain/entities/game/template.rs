//! Template de jeu (catalogue) : reference Docker image + schema config UX.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type d'un champ configurable cote UX (game-portal page).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFieldType {
    Text,
    Number,
    Enum,
    Boolean,
}

/// Definition d'un champ configurable par l'admin du jeu.
/// Sert au front pour generer dynamiquement le formulaire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: ConfigFieldType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
}

/// Protocole reseau du port jeu (TCP : Minecraft, Terraria... ; UDP :
/// Valheim, Factorio, Palworld...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortProtocol {
    Tcp,
    Udp,
}

impl PortProtocol {
    pub fn from_str(s: &str) -> Self {
        match s {
            "udp" => Self::Udp,
            _ => Self::Tcp,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
        }
    }
}

/// Template de jeu — entree du catalogue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameTemplate {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub image: String,
    pub category: Option<String>,
    pub icon: Option<String>,
    pub accent_color: Option<String>,
    pub cover_image_url: Option<String>,
    pub container_port: u16,
    pub port_protocol: PortProtocol,
    /// Path interne du container ou le volume nomme est monte. Defaut /data
    /// (Minecraft). Override par jeu (Terraria : /root/.local/share/...).
    pub volume_path: String,
    /// Si TRUE, l'API ne passe pas --user 1000:1000 et laisse l'image
    /// utiliser son user par defaut (root pour Terraria/Valheim/Factorio).
    pub run_as_root: bool,
    pub default_memory_mb: i32,
    pub min_memory_mb: i32,
    pub max_memory_mb: i32,
    pub default_env: serde_json::Value,
    pub config_schema: Vec<ConfigField>,
    pub supports_rcon: bool,
    pub supports_mods: bool,
    pub idle_shutdown_days: i32,
    /// Fichiers a poser sur le volume avant `start_container`. Le content
    /// peut contenir des `{{KEY}}` substitues par les env vars effectives.
    /// Vide pour la majorite des jeux (env vars suffisent).
    pub init_files: Vec<InitFile>,
    /// Override de la commande Docker (CMD). JSON array. Templatable via
    /// `{{KEY}}`. None = utilise le CMD de l'image (cas par defaut).
    pub command: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fichier seed a uploader dans le container apres create / avant start.
/// Le `content` est un template avec `{{KEY}}` substitues a partir des env
/// vars (defaults du template + overrides utilisateur). Cf. `init_files`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitFile {
    pub path: String,
    pub content: String,
}

impl GameTemplate {
    /// Verifie qu'un override de memoire est dans les bornes du template.
    pub fn validate_memory(&self, requested_mb: i32) -> Result<(), String> {
        if requested_mb < self.min_memory_mb {
            return Err(format!(
                "memoire trop basse: {} Mo < min {} Mo",
                requested_mb, self.min_memory_mb
            ));
        }
        if requested_mb > self.max_memory_mb {
            return Err(format!(
                "memoire trop haute: {} Mo > max {} Mo",
                requested_mb, self.max_memory_mb
            ));
        }
        Ok(())
    }

    /// Cherche la definition d'un champ config par sa key.
    pub fn find_field(&self, key: &str) -> Option<&ConfigField> {
        self.config_schema.iter().find(|f| f.key == key)
    }

    /// Valide une valeur (string brute, telle que stockee) contre la
    /// definition de champ du template : bornes numeriques (min/max),
    /// options autorisees (enum), et longueur max (text). Une key absente
    /// du schema est acceptee (la validation de key reste a l'appelant).
    pub fn validate_config_value(&self, key: &str, value: &str) -> Result<(), String> {
        let field = match self.find_field(key) {
            Some(f) => f,
            // SECURITE : une cle absente du schema du template est REJETEE (avant,
            // acceptee -> injectait n'importe quelle variable d'env dans le
            // conteneur, ex. LD_PRELOAD / JAVA_TOOL_OPTIONS -> RCE conteneur).
            None => {
                return Err(format!(
                    "'{key}': cle de configuration inconnue pour ce template"
                ))
            }
        };
        match field.field_type {
            ConfigFieldType::Number => {
                let num: f64 = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("'{key}': valeur numerique attendue, recu '{value}'"))?;
                if let Some(min) = field.min {
                    if num < min {
                        return Err(format!("'{key}': {num} < min {min}"));
                    }
                }
                if let Some(max) = field.max {
                    if num > max {
                        return Err(format!("'{key}': {num} > max {max}"));
                    }
                }
            }
            ConfigFieldType::Enum => {
                if let Some(opts) = &field.options {
                    if !opts.iter().any(|o| o == value) {
                        return Err(format!(
                            "'{key}': valeur '{value}' non autorisee (options: {})",
                            opts.join(", ")
                        ));
                    }
                }
            }
            ConfigFieldType::Boolean => {
                if !matches!(value, "true" | "false") {
                    return Err(format!(
                        "'{key}': booleen attendu ('true'/'false'), recu '{value}'"
                    ));
                }
            }
            ConfigFieldType::Text => {
                if let Some(max_len) = field.max_length {
                    if value.chars().count() > max_len as usize {
                        return Err(format!(
                            "'{key}': longueur {} > max {max_len}",
                            value.chars().count()
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpl(fields: Vec<ConfigField>) -> GameTemplate {
        GameTemplate {
            id: Uuid::nil(),
            slug: "t".into(),
            name: "t".into(),
            description: None,
            image: "img".into(),
            category: None,
            icon: None,
            accent_color: None,
            cover_image_url: None,
            container_port: 25565,
            port_protocol: PortProtocol::Tcp,
            volume_path: "/data".into(),
            run_as_root: false,
            default_memory_mb: 1024,
            min_memory_mb: 512,
            max_memory_mb: 4096,
            default_env: serde_json::json!({}),
            config_schema: fields,
            supports_rcon: true,
            supports_mods: false,
            idle_shutdown_days: 7,
            init_files: vec![],
            command: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn field(key: &str, ty: ConfigFieldType) -> ConfigField {
        ConfigField {
            key: key.into(),
            label: key.into(),
            field_type: ty,
            default: None,
            options: None,
            min: None,
            max: None,
            max_length: None,
        }
    }

    #[test]
    fn unknown_key_is_rejected() {
        // SECURITE : une cle hors schema est refusee (avant, acceptee -> env
        // arbitraire injecte dans le conteneur).
        let t = tmpl(vec![]);
        assert!(t.validate_config_value("WHATEVER", "x").is_err());
        assert!(t.validate_config_value("LD_PRELOAD", "/evil.so").is_err());
    }

    #[test]
    fn number_within_bounds_passes_and_out_of_range_rejected() {
        let mut f = field("MAX_PLAYERS", ConfigFieldType::Number);
        f.min = Some(1.0);
        f.max = Some(20.0);
        let t = tmpl(vec![f]);
        assert!(t.validate_config_value("MAX_PLAYERS", "10").is_ok());
        assert!(t.validate_config_value("MAX_PLAYERS", "0").is_err());
        assert!(t.validate_config_value("MAX_PLAYERS", "21").is_err());
        assert!(t.validate_config_value("MAX_PLAYERS", "abc").is_err());
    }

    #[test]
    fn enum_option_validated() {
        let mut f = field("DIFFICULTY", ConfigFieldType::Enum);
        f.options = Some(vec!["easy".into(), "hard".into()]);
        let t = tmpl(vec![f]);
        assert!(t.validate_config_value("DIFFICULTY", "easy").is_ok());
        assert!(t.validate_config_value("DIFFICULTY", "extreme").is_err());
    }

    #[test]
    fn text_max_length_enforced() {
        let mut f = field("MOTD", ConfigFieldType::Text);
        f.max_length = Some(5);
        let t = tmpl(vec![f]);
        assert!(t.validate_config_value("MOTD", "hello").is_ok());
        assert!(t.validate_config_value("MOTD", "helloo").is_err());
    }

    #[test]
    fn boolean_validated() {
        let t = tmpl(vec![field("PVP", ConfigFieldType::Boolean)]);
        assert!(t.validate_config_value("PVP", "true").is_ok());
        assert!(t.validate_config_value("PVP", "false").is_ok());
        assert!(t.validate_config_value("PVP", "yes").is_err());
    }
}
