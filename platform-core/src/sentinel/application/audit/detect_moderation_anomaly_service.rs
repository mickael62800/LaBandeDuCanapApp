use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::moderation_anomaly::ModerationAnomaly;
use crate::sentinel::ports::inbound::audit::detect_moderation_anomaly::DetectAnomalyCommand;
use crate::sentinel::ports::inbound::audit::detect_moderation_anomaly::DetectModerationAnomalyUseCase;
use crate::sentinel::ports::outbound::audit::moderation_anomaly_counter::ModerationAnomalyCounter;

/// Service de detection d'anomalie de moderation.
///
/// Mirroir exact de l'ancien `AnomalyDetector::record` du bot : pour chaque
/// evenement, on incremente le compteur fenetre (adapter serveur) et on compare
/// au seuil de la categorie ; au franchissement, on reset le compteur (anti
/// boucle) et on renvoie l'alerte. Le `increment > 1` (purge bulk) rejoue la
/// meme logique evenement par evenement et s'arrete au premier declenchement.
pub struct DetectModerationAnomalyService {
    counter: Arc<dyn ModerationAnomalyCounter>,
}

impl DetectModerationAnomalyService {
    pub fn new(counter: Arc<dyn ModerationAnomalyCounter>) -> Self {
        Self { counter }
    }
}

#[async_trait]
impl DetectModerationAnomalyUseCase for DetectModerationAnomalyService {
    async fn detect(&self, command: DetectAnomalyCommand) -> Option<ModerationAnomaly> {
        let threshold = command.thresholds.threshold_for(&command.category);
        let steps = command.increment.max(1);

        for _ in 0..steps {
            let count = self
                .counter
                .record(&command.guild_id, &command.category, command.window_secs)
                .await;

            if count >= threshold {
                self.counter
                    .reset(&command.guild_id, &command.category)
                    .await;
                return Some(ModerationAnomaly {
                    anomaly_type: format!("mass_{}", command.category),
                    count,
                    window_secs: command.window_secs,
                });
            }
        }

        None
    }
}

#[cfg(test)]
#[path = "tests/detect_moderation_anomaly.rs"]
mod tests;
