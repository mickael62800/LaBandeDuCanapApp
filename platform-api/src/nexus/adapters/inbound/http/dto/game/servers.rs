use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use platform_core::nexus::domain::entities::game::server::{GameServer, GameServerStatus};
use platform_core::nexus::ports::inbound::game::manage_game_servers::GameServerDetail;
use platform_core::nexus::ports::outbound::game::container_runtime::ContainerStats;

/// Deserialise une map de config en TOLERANT les scalaires JSON non-chaine.
///
/// La config est stockee et validee comme du texte (`HashMap<String, String>`),
/// mais le formulaire web envoie naturellement un champ entier comme un nombre
/// JSON (`"PLAYERS": 10`) et une case a cocher comme un booleen. Sans cette
/// conversion, serde rejetait tout le corps en 422 (« invalid type: integer,
/// expected a string »). On accepte chaine/nombre/booleen et on normalise en
/// chaine ; `null` est ignore (champ laisse au defaut) ; objet/tableau restent
/// une vraie erreur.
fn deserialize_config_map<'de, D>(deserializer: D) -> Result<HashMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let raw: HashMap<String, serde_json::Value> = HashMap::deserialize(deserializer)?;
    let mut out = HashMap::with_capacity(raw.len());
    for (k, v) in raw {
        let s = match v {
            serde_json::Value::String(s) => s,
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => continue,
            other => {
                return Err(serde::de::Error::custom(format!(
                    "valeur de config invalide pour `{k}` : scalaire attendu, recu {other}"
                )))
            }
        };
        out.insert(k, s);
    }
    Ok(out)
}

/// Corps de création d'un serveur de jeu.
///
/// `template_slug` et `name` sont obligatoires. Les valeurs mémoire et CPU
/// sont contrôlées ensuite par le template et le quota de la guilde.
#[derive(Debug, Deserialize)]
pub struct CreateGameServerDto {
    pub template_slug: String,
    pub name: String,
    /// Memoire en Mo (sinon default du template).
    pub memory_mb: Option<i32>,
    /// Plafond CPU en coeurs (ex: 2.0). Vide = defaut de l'adapter.
    pub cpu_limit: Option<f64>,
    pub owner_user_id: String,
    /// Overrides initiaux (key/value SCREAMING_SNAKE).
    #[serde(default, deserialize_with = "deserialize_config_map")]
    pub config: HashMap<String, String>,
    /// Reglement de la soiree. Vide = aucun. Reproduit mot pour mot sous
    /// l'annonce ; Atrium le recoit comme contexte mais ne le reformule pas.
    #[serde(default)]
    pub rules: Option<String>,
    /// Delai (jours) avant la revelation de l'IP. Vide = defaut de la guild
    /// (`ip_reveal_default_days`). 0 = pas de revelation programmee.
    pub ip_reveal_days: Option<i32>,
}

/// Remplacements de configuration appliqués à un serveur existant.
///
/// Les valeurs acceptées sont des scalaires normalisés en texte. Les objets,
/// tableaux et valeurs nulles ne sont pas des réglages valides.
#[derive(Debug, Deserialize)]
pub struct UpdateConfigDto {
    #[serde(deserialize_with = "deserialize_config_map")]
    pub config: HashMap<String, String>,
}

/// Commande envoyée au serveur via RCON.
#[derive(Debug, Deserialize)]
pub struct RconCommandDto {
    pub command: String,
}

/// Exécution d'une commande DU CATALOGUE.
///
/// Le navigateur envoie une clé et des paramètres, jamais une commande : le
/// gabarit est retrouvé et composé côté serveur.
#[derive(Debug, Deserialize)]
pub struct CatalogCommandDto {
    #[serde(default)]
    pub params: std::collections::HashMap<String, String>,
}

/// Un joueur actuellement connecté au serveur de jeu.
#[derive(Debug, Serialize)]
pub struct OnlinePlayerDto {
    pub name: String,
    /// Identifiant vérifiable dans le jeu quand le serveur l'expose (SteamID64
    /// pour Palworld). C'est lui que prennent les commandes de modération.
    pub game_player_id: Option<String>,
}

/// Vue publique côté administration d'un serveur de jeu.
///
/// Le runtime et le mot de passe RCON ne sont jamais exposés dans ce DTO.
/// `public_host` est réservé à la surface authentifiée d'administration.
#[derive(Debug, Serialize)]
pub struct GameServerDto {
    pub id: Uuid,
    pub guild_id: String,
    pub template_id: Uuid,
    pub name: String,
    pub status: String,
    pub host_port: Option<u16>,
    pub rcon_port: Option<u16>,
    pub allocated_memory_mb: i32,
    pub cpu_limit: Option<f64>,
    pub owner_user_id: String,
    pub last_active_at: Option<DateTime<Utc>>,
    pub last_player_count: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
    // Session Discord (evenement de serveur).
    pub announcement_posted_at: Option<String>,
    pub rules: Option<String>,
    pub channel_name_registration: Option<String>,
    pub channel_name_private: Option<String>,
    pub channel_name_voice: Option<String>,
    pub text_channel_id: Option<String>,
    pub voice_channel_id: Option<String>,
    pub ip_reveal_at: Option<DateTime<Utc>>,
    pub ip_revealed: bool,
    /// Heure de fin annoncee de la session.
    pub closes_at: Option<DateTime<Utc>>,
    /// Ce que la session annonce : `waiting` | `open` | `closed`. Calcule par
    /// le domaine a partir de la fenetre horaire ET du conteneur.
    pub display_state: String,
    /// Hote public du serveur, tel qu'il sera annonce aux joueurs.
    ///
    /// Renseigne independamment de `ip_revealed` : la revelation programmee
    /// concerne les JOUEURS, pas l'administration. Un admin qui prepare une
    /// session a besoin de l'adresse pour la tester avant de l'ouvrir.
    ///
    /// Cette route est derriere la cle d'API, jamais exposee publiquement —
    /// c'est `PublicGameServerDto` qui gere la revelation cote joueurs.
    pub public_host: Option<String>,
}

impl From<GameServer> for GameServerDto {
    fn from(s: GameServer) -> Self {
        Self {
            id: s.id,
            guild_id: s.guild_id,
            template_id: s.template_id,
            name: s.name,
            status: status_str(s.status).to_string(),
            host_port: s.host_port,
            rcon_port: s.rcon_port,
            allocated_memory_mb: s.allocated_memory_mb,
            cpu_limit: s.cpu_limit,
            owner_user_id: s.owner_user_id,
            last_active_at: s.last_active_at,
            last_player_count: s.last_player_count,
            last_error: s.last_error,
            created_at: s.created_at,
            started_at: s.started_at,
            stopped_at: s.stopped_at,
            announcement_posted_at: s.announcement_posted_at.map(|d| d.to_rfc3339()),
            rules: s.rules,
            channel_name_registration: s.channel_name_registration,
            channel_name_private: s.channel_name_private,
            channel_name_voice: s.channel_name_voice,
            text_channel_id: s.text_channel_id,
            voice_channel_id: s.voice_channel_id,
            ip_reveal_at: s.ip_reveal_at,
            ip_revealed: s.ip_revealed,
            closes_at: s.closes_at,
            display_state:
                platform_core::nexus::domain::entities::game::session_state::session_display_state(
                    s.status,
                    s.ip_reveal_at,
                    s.closes_at,
                    Utc::now(),
                )
                .as_str()
                .to_string(),
            // L'hote ne vit pas sur l'entite : il est commun a la guild et
            // releve de la configuration. Renseigne par `avec_hote`.
            public_host: None,
        }
    }
}

impl GameServerDto {
    /// Renseigne l'hote public. Vide = non configure, on laisse `None` plutot
    /// qu'une chaine vide, pour que le front distingue les deux cas.
    pub fn avec_hote(mut self, hote: Option<&str>) -> Self {
        self.public_host = hote.filter(|h| !h.trim().is_empty()).map(str::to_string);
        self
    }
}

fn status_str(s: GameServerStatus) -> &'static str {
    s.as_str()
}

#[derive(Debug, Serialize)]
pub struct GameServerDetailDto {
    pub server: GameServerDto,
    pub config: HashMap<String, String>,
}

impl From<GameServerDetail> for GameServerDetailDto {
    fn from(d: GameServerDetail) -> Self {
        Self {
            server: GameServerDto::from(d.server),
            config: d.config,
        }
    }
}

impl GameServerDetailDto {
    pub fn avec_hote(mut self, hote: Option<&str>) -> Self {
        self.server = self.server.avec_hote(hote);
        self
    }
}

#[derive(Debug, Serialize)]
pub struct GameServerStatsDto {
    pub cpu_percent: f64,
    pub memory_used_mb: u64,
    pub memory_limit_mb: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    /// Débit réseau, dérivé de l'échantillon précédent du contrôle de santé.
    /// `null` tant qu'aucune comparaison honnête n'est possible.
    pub network_rx_bytes_per_sec: Option<u64>,
    pub network_tx_bytes_per_sec: Option<u64>,
    /// Temps de réponse du jeu à la dernière commande de contrôle. C'est le
    /// signal de lag le plus direct : un serveur peut ramer à 30 % de CPU.
    pub rcon_latency_ms: Option<i32>,
}

impl From<ContainerStats> for GameServerStatsDto {
    fn from(s: ContainerStats) -> Self {
        Self {
            cpu_percent: s.cpu_percent,
            memory_used_mb: s.memory_used_bytes / (1024 * 1024),
            memory_limit_mb: s.memory_limit_bytes / (1024 * 1024),
            network_rx_bytes: s.network_rx_bytes,
            network_tx_bytes: s.network_tx_bytes,
            // Renseignes par le handler : le debit et la latence viennent de
            // la ligne serveur, pas des statistiques Docker de l'instant.
            network_rx_bytes_per_sec: None,
            network_tx_bytes_per_sec: None,
            rcon_latency_ms: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RconCommandResponseDto {
    pub response: String,
}
