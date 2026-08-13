use platform_core::sentinel::domain::entities::audit::moderation_anomaly::AnomalyThresholds;
use platform_core::sentinel::domain::entities::audit::moderation_anomaly::ModerationAnomaly;
use platform_core::sentinel::ports::inbound::audit::detect_moderation_anomaly::DetectAnomalyCommand;
use serde::Deserialize;
use serde::Serialize;

fn default_increment() -> usize {
    1
}

fn default_window_secs() -> u64 {
    60
}

/// Requete de detection d'anomalie envoyee par le bot a chaque evenement de
/// moderation (ban/kick/delete/role_change). Les seuils sont resolus per-guild
/// cote bot (depuis bot_guild_config) et transmis ici.
#[derive(Debug, Deserialize)]
pub struct DetectAnomalyRequestDto {
    pub guild_id: String,
    pub category: String,
    #[serde(default = "default_increment")]
    pub increment: usize,
    #[serde(default = "default_window_secs")]
    pub window_secs: u64,
    pub mass_ban: usize,
    pub mass_delete: usize,
    pub mass_role_change: usize,
}

impl From<DetectAnomalyRequestDto> for DetectAnomalyCommand {
    fn from(dto: DetectAnomalyRequestDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            category: dto.category,
            increment: dto.increment,
            window_secs: dto.window_secs,
            thresholds: AnomalyThresholds {
                mass_ban: dto.mass_ban,
                mass_delete: dto.mass_delete,
                mass_role_change: dto.mass_role_change,
            },
        }
    }
}

/// Alerte d'anomalie a afficher cote bot.
#[derive(Debug, Serialize)]
pub struct AnomalyAlertDto {
    pub anomaly_type: String,
    pub count: usize,
    pub window_secs: u64,
}

impl From<ModerationAnomaly> for AnomalyAlertDto {
    fn from(a: ModerationAnomaly) -> Self {
        Self {
            anomaly_type: a.anomaly_type,
            count: a.count,
            window_secs: a.window_secs,
        }
    }
}

/// Reponse : `alert` non nul si une anomalie a ete decidee cote serveur.
#[derive(Debug, Serialize)]
pub struct DetectAnomalyResponseDto {
    pub alert: Option<AnomalyAlertDto>,
}
