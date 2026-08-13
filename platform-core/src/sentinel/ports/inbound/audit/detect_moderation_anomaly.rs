use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::moderation_anomaly::AnomalyThresholds;
use crate::sentinel::domain::entities::audit::moderation_anomaly::ModerationAnomaly;

/// Commande de detection d'anomalie de moderation.
pub struct DetectAnomalyCommand {
    pub guild_id: String,
    /// Categorie d'evenement : `ban`, `kick`, `delete`, `role_change`.
    pub category: String,
    /// Nombre d'evenements a enregistrer (>=1). Vaut > 1 pour les purges bulk.
    pub increment: usize,
    /// Taille de la fenetre glissante en secondes.
    pub window_secs: u64,
    /// Seuils resolus per-guild par l'appelant.
    pub thresholds: AnomalyThresholds,
}

/// Use case : enregistre un evenement de moderation cote serveur et decide
/// s'il constitue une anomalie (mass ban/delete/role). Retourne l'alerte a
/// afficher si le seuil est franchi, sinon `None`.
#[async_trait]
pub trait DetectModerationAnomalyUseCase: Send + Sync {
    async fn detect(&self, command: DetectAnomalyCommand) -> Option<ModerationAnomaly>;
}
