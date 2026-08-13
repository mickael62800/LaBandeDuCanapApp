use std::hash::Hash;
use std::time::Duration;

use crate::sentinel::domain::services::sliding_window::SlidingWindow;

/// Détecteur de raid basé sur le nombre de joins dans une fenêtre de temps.
/// Générique sur la clé `K` (l'adaptateur fournit son type d'identifiant de
/// serveur, ex. `GuildId`) pour rester pur — le core ne connaît pas Discord.
/// Stockage : `SlidingWindow` partagée (politique de purge unique).
pub struct RaidDetector<K: Eq + Hash + Clone> {
    joins: SlidingWindow<K>,
    threshold: u64,
    window: Duration,
}

impl<K: Eq + Hash + Clone> RaidDetector<K> {
    pub fn new(threshold: u64, window_secs: u64) -> Self {
        Self {
            joins: SlidingWindow::new(),
            threshold,
            window: Duration::from_secs(window_secs),
        }
    }

    /// Enregistre un join et retourne `true` si un raid est détecté.
    pub fn record_join(&self, key: K) -> bool {
        let count = self.joins.record(key, self.window);
        self.joins.prune_if_larger(1000, self.window * 2);
        count as u64 >= self.threshold
    }

    /// Retourne le nombre de joins récents pour une clé.
    pub fn recent_joins(&self, key: K) -> u64 {
        self.joins.count(&key, self.window) as u64
    }

    /// Réinitialise les compteurs d'une clé (après lockdown par ex).
    pub fn reset(&self, key: K) {
        self.joins.clear(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_raid_below_threshold() {
        let detector = RaidDetector::<u64>::new(5, 10);
        for _ in 0..4 {
            assert!(!detector.record_join(1));
        }
    }

    #[test]
    fn test_raid_at_threshold() {
        let detector = RaidDetector::<u64>::new(3, 10);
        assert!(!detector.record_join(1));
        assert!(!detector.record_join(1));
        assert!(detector.record_join(1)); // 3eme = raid
    }

    #[test]
    fn test_different_guilds_independent() {
        let detector = RaidDetector::<u64>::new(2, 10);
        assert!(!detector.record_join(1));
        assert!(!detector.record_join(2));
        assert!(detector.record_join(1)); // 2eme pour A
        assert!(detector.record_join(2)); // 2eme pour B
    }

    #[test]
    fn test_reset_clears_count() {
        let detector = RaidDetector::<u64>::new(3, 10);
        detector.record_join(1);
        detector.record_join(1);
        detector.reset(1);
        assert_eq!(detector.recent_joins(1), 0);
        assert!(!detector.record_join(1)); // repart de 1
    }
}
