use std::time::Duration;
use std::time::Instant;

use async_trait::async_trait;
use dashmap::DashMap;

use platform_core::sentinel::ports::outbound::audit::moderation_anomaly_counter::ModerationAnomalyCounter;

/// Compteur d'anomalies de moderation a fenetre glissante, en memoire serveur.
///
/// Remonte depuis l'ancien `AnomalyDetector` du bot (le CALCUL uniquement). On
/// stocke les horodatages recents par `(guild, categorie)` ; `record` purge les
/// entrees hors fenetre, borne la taille du buffer, puis retourne le compte.
/// La DECISION (seuil, reset apres alerte) vit dans le service coeur.
pub struct InMemoryAnomalyCounter {
    counters: DashMap<(String, String), Vec<Instant>>,
    /// Taille max du buffer d'horodatages avant eviction.
    max_buffer_size: usize,
    /// Nombre d'horodatages conserves apres eviction (les plus recents).
    eviction_target: usize,
}

impl InMemoryAnomalyCounter {
    pub fn new(max_buffer_size: usize, eviction_target: usize) -> Self {
        // Garde-fous identiques a l'ancien detecteur : cible d'eviction >= 1 et
        // jamais superieure a la taille max du buffer.
        let eviction_target = eviction_target.max(1);
        let max_buffer_size = max_buffer_size.max(eviction_target);
        Self {
            counters: DashMap::new(),
            max_buffer_size,
            eviction_target,
        }
    }
}

#[async_trait]
impl ModerationAnomalyCounter for InMemoryAnomalyCounter {
    async fn record(&self, guild_id: &str, category: &str, window_secs: u64) -> usize {
        let now = Instant::now();
        let window = Duration::from_secs(window_secs);
        let key = (guild_id.to_string(), category.to_string());
        let mut entry = self.counters.entry(key).or_default();
        let timestamps = entry.value_mut();

        // Nettoyer hors fenetre.
        timestamps.retain(|t| now.duration_since(*t) < window);
        // Securite : borner la taille du vecteur.
        if timestamps.len() > self.max_buffer_size {
            timestamps.drain(0..timestamps.len() - self.eviction_target);
        }
        timestamps.push(now);

        timestamps.len()
    }

    async fn reset(&self, guild_id: &str, category: &str) {
        let key = (guild_id.to_string(), category.to_string());
        if let Some(mut entry) = self.counters.get_mut(&key) {
            entry.value_mut().clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn record_increments_within_window() {
        let counter = InMemoryAnomalyCounter::new(500, 100);
        assert_eq!(counter.record("1", "ban", 60).await, 1);
        assert_eq!(counter.record("1", "ban", 60).await, 2);
        assert_eq!(counter.record("1", "ban", 60).await, 3);
    }

    #[tokio::test]
    async fn reset_clears_count() {
        let counter = InMemoryAnomalyCounter::new(500, 100);
        counter.record("1", "ban", 60).await;
        counter.record("1", "ban", 60).await;
        counter.reset("1", "ban").await;
        assert_eq!(counter.record("1", "ban", 60).await, 1);
    }

    #[tokio::test]
    async fn categories_and_guilds_independent() {
        let counter = InMemoryAnomalyCounter::new(500, 100);
        counter.record("1", "ban", 60).await;
        assert_eq!(counter.record("1", "delete", 60).await, 1);
        assert_eq!(counter.record("2", "ban", 60).await, 1);
    }

    #[tokio::test]
    async fn zero_window_evicts_previous() {
        // Fenetre de 0s : chaque record purge les precedents (hors fenetre).
        let counter = InMemoryAnomalyCounter::new(500, 100);
        counter.record("1", "ban", 0).await;
        assert_eq!(counter.record("1", "ban", 0).await, 1);
    }
}
