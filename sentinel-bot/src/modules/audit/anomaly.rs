use serde::Deserialize;

/// Seuils de detection d'anomalie, resolus per-guild depuis bot_guild_config
/// et transmis a l'API qui decide (cf. `anomaly_thresholds_for`). Le type et
/// ses defauts vivent dans le core (source unique — l'API utilise le meme).
pub use platform_core::sentinel::domain::entities::audit::moderation_anomaly::AnomalyThresholds;

/// Alerte d'anomalie decidee par l'API et renvoyee au bot pour affichage.
///
/// La DECISION (comptage fenetre + seuil + reset) est desormais server-side
/// (`DetectModerationAnomaly` cote sentinel-api). Le bot ne fait qu'afficher
/// l'embed URGENT a partir de cette alerte.
#[derive(Debug, Clone, Deserialize)]
pub struct AnomalyAlert {
    pub anomaly_type: String,
    pub count: usize,
    pub window_secs: u64,
}
