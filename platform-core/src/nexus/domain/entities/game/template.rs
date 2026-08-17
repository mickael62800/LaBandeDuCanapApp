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
    /// Section d'affichage du formulaire. Un jeu expose jusqu'a une
    /// cinquantaine de reglages : sans regroupement, la page est inutilisable.
    ///
    /// Absent du schema = pas de section connue, le front regroupe le reste
    /// sous une rubrique generale. Ce champ etait ecrit dans les migrations
    /// mais manquait ici : la lecture le supprimait donc AVANT que le front ne
    /// le voie, et toutes les sections etaient perdues.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Aide affichee sous le champ : ce que le reglage fait.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Ce que le reglage CASSE, par opposition a `description`. Affiche
    /// distinctement : noye dans le texte courant, il ne serait pas lu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
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
    /// Commandes d'administration proposees a l'ecran pour ce jeu. Vide tant
    /// qu'aucun catalogue n'a ete ecrit : le jeu n'expose alors que la console
    /// libre, comme avant.
    #[serde(default)]
    pub command_schema: Vec<crate::nexus::domain::entities::game::command::GameCommand>,
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
            command_schema: vec![],
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
            group: None,
            description: None,
            warning: None,
            default: None,
            options: None,
            min: None,
            max: None,
            max_length: None,
        }
    }

    #[test]
    fn presentation_fields_survive_a_round_trip() {
        // Ces trois cles sont ecrites dans les migrations mais n'existaient pas
        // sur `ConfigField` : la lecture du schema les supprimait, et le front
        // recevait des reglages sans section, sans aide et sans avertissement.
        let brut = serde_json::json!({
            "key": "ADMIN_PASSWORD",
            "label": "Mot de passe administrateur",
            "type": "text",
            "group": "Acces",
            "description": "Laisser vide pour desactiver.",
            "warning": "Donne le controle TOTAL du serveur."
        });

        let champ: ConfigField = serde_json::from_value(brut).expect("schema lisible");
        assert_eq!(champ.group.as_deref(), Some("Acces"));
        assert_eq!(
            champ.description.as_deref(),
            Some("Laisser vide pour desactiver.")
        );
        assert_eq!(
            champ.warning.as_deref(),
            Some("Donne le controle TOTAL du serveur.")
        );

        // Et elles doivent ressortir telles quelles vers le front.
        let renvoye = serde_json::to_value(&champ).expect("schema serialisable");
        assert_eq!(renvoye["group"], "Acces");
        assert_eq!(renvoye["warning"], "Donne le controle TOTAL du serveur.");
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
