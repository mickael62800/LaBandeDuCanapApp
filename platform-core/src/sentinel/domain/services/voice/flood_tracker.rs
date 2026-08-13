use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::sentinel::domain::services::sliding_window::SlidingWindow;

const DEFAULT_MAX_MESSAGES: u64 = 5;
const DEFAULT_TIME_WINDOW_SECS: u64 = 5;

/// Détecteur de flood par fenêtre glissante, par couple (salon, utilisateur).
/// Générique sur les clés `C` (salon) et `U` (utilisateur) — le core ne
/// connaît pas Discord. Seuils reconfigurables à chaud depuis la config API.
/// Stockage : `SlidingWindow` partagée avec purge amortie (les couples
/// inactifs sont éjectés — corrige une fuite mémoire non bornée).
pub struct FloodTracker<C: Eq + Hash + Clone, U: Eq + Hash + Clone> {
    map: SlidingWindow<(C, U)>,
    max_messages: AtomicU64,
    time_window_secs: AtomicU64,
}

impl<C: Eq + Hash + Clone, U: Eq + Hash + Clone> FloodTracker<C, U> {
    pub fn new() -> Self {
        Self {
            map: SlidingWindow::new(),
            max_messages: AtomicU64::new(DEFAULT_MAX_MESSAGES),
            time_window_secs: AtomicU64::new(DEFAULT_TIME_WINDOW_SECS),
        }
    }

    /// Met a jour les seuils depuis la config API.
    pub fn set_thresholds(&self, max_messages: u64, time_window_secs: u64) {
        self.max_messages.store(max_messages, Ordering::Relaxed);
        self.time_window_secs
            .store(time_window_secs, Ordering::Relaxed);
    }

    fn window(&self) -> Duration {
        Duration::from_secs(self.time_window_secs.load(Ordering::Relaxed))
    }

    /// Enregistre un message. Retourne true si flood detecte.
    pub fn record_message(&self, channel_id: C, user_id: U) -> bool {
        let window = self.window();
        let max = self.max_messages.load(Ordering::Relaxed) as usize;
        let count = self.map.record((channel_id, user_id), window);
        self.map.prune_if_larger(1000, window * 2);
        count >= max
    }

    /// Nettoie le compteur pour un utilisateur dans un channel.
    pub fn clear(&self, channel_id: C, user_id: U) {
        self.map.clear(&(channel_id, user_id));
    }
}

impl<C: Eq + Hash + Clone, U: Eq + Hash + Clone> Default for FloodTracker<C, U> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Tracker = FloodTracker<u64, u64>;

    #[test]
    fn test_no_flood_below_threshold() {
        let tracker = Tracker::new();
        for _ in 0..(DEFAULT_MAX_MESSAGES - 1) {
            assert!(!tracker.record_message(1, 1));
        }
    }

    #[test]
    fn test_flood_at_threshold() {
        let tracker = Tracker::new();
        for i in 0..DEFAULT_MAX_MESSAGES {
            let result = tracker.record_message(1, 1);
            if i < DEFAULT_MAX_MESSAGES - 1 {
                assert!(!result);
            } else {
                assert!(result);
            }
        }
    }

    #[test]
    fn test_different_users_independent() {
        let tracker = Tracker::new();
        for _ in 0..(DEFAULT_MAX_MESSAGES - 1) {
            tracker.record_message(1, 1);
        }
        assert!(!tracker.record_message(1, 2));
    }

    #[test]
    fn test_clear_resets() {
        let tracker = Tracker::new();
        for _ in 0..(DEFAULT_MAX_MESSAGES - 1) {
            tracker.record_message(1, 1);
        }
        tracker.clear(1, 1);
        assert!(!tracker.record_message(1, 1));
    }
}
