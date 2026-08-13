//! Slowmode adaptatif — logique PURE (fenêtre glissante d'activité + décision
//! d'activation/désactivation). Générique sur la clé `K` (l'adaptateur fournit
//! son `ChannelId`) pour rester sans dépendance Discord. La pose/retrait effectif
//! du slowmode Discord reste dans l'adaptateur.
//! Stockage : `SlidingWindow` partagée ; la DÉCISION (seuils, ensembles
//! activating/active) reste ici.

use std::hash::Hash;
use std::time::Duration;

use dashmap::DashSet;

use crate::sentinel::domain::services::sliding_window::SlidingWindow;

/// Tracker d'activité par salon pour le slowmode adaptatif.
pub struct SlowmodeTracker<K: Eq + Hash + Clone> {
    /// clé (salon) -> timestamps des messages récents
    counters: SlidingWindow<K>,
    /// salons en cours d'activation (évite les activations multiples)
    activating: DashSet<K>,
    /// salons dont l'automod a RÉELLEMENT activé un slowmode (pour ne
    /// désactiver que ceux-là, jamais un slowmode manuel de modérateur).
    active: DashSet<K>,
    window: Duration,
}

impl<K: Eq + Hash + Clone> SlowmodeTracker<K> {
    pub fn new(window_secs: u64) -> Self {
        Self {
            counters: SlidingWindow::new(),
            activating: DashSet::new(),
            active: DashSet::new(),
            window: Duration::from_secs(window_secs),
        }
    }

    /// Marque un salon comme ayant un slowmode adaptatif actif (à appeler après
    /// avoir posé le slowmode). Ne touche PAS au compteur (sinon le salon
    /// disparaît du suivi et ne peut plus être désactivé).
    pub fn mark_active(&self, key: K) {
        self.active.insert(key);
    }

    /// Enregistre un message et retourne le nombre de messages dans la fenêtre.
    pub fn record_message(&self, key: K) -> usize {
        self.counters.record(key, self.window)
    }

    /// Vérifie si le seuil d'activation est atteint.
    pub fn should_activate(&self, key: K, threshold: usize) -> bool {
        self.counters.count(&key, self.window) >= threshold
    }

    /// Tente de démarrer l'activation du slowmode. Retourne true si ok (pas déjà en cours).
    pub fn try_start_activation(&self, key: K) -> bool {
        self.activating.insert(key)
    }

    /// Termine l'activation du slowmode.
    pub fn finish_activation(&self, key: K) {
        self.activating.remove(&key);
    }

    /// Reset le compteur d'un salon.
    pub fn reset(&self, key: K) {
        self.counters.clear(&key);
    }

    /// Supprime les salons inactifs (pas de message depuis > 2x la fenêtre).
    pub fn cleanup(&self) {
        self.counters.prune(self.window * 2);
    }

    /// Retourne le nombre de salons actuellement suivis.
    pub fn tracked_channels(&self) -> usize {
        self.counters.len()
    }

    /// Retourne les salons dont le slowmode a été activé mais dont l'activité
    /// est retombée sous le seuil (aucun message dans la fenêtre).
    /// Ces salons devraient avoir leur slowmode désactivé.
    pub fn channels_to_deactivate(&self, threshold: usize) -> Vec<K> {
        let floor = (threshold / 2).max(1);
        // On n'examine QUE les salons activés par l'automod (jamais les
        // slowmodes manuels des modos).
        let mut out = Vec::new();
        for ch in self.active.iter() {
            let ch = ch.key().clone();
            if self.counters.count(&ch, self.window) < floor {
                out.push(ch);
            }
        }
        // Ces salons vont être désactivés : on les retire de l'ensemble actif.
        for ch in &out {
            self.active.remove(ch);
        }
        out
    }

    /// Retourne le nombre de messages dans la fenêtre pour un salon.
    pub fn count(&self, key: K) -> usize {
        self.counters.count(&key, self.window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_count() {
        let tracker = SlowmodeTracker::<u64>::new(60);
        assert_eq!(tracker.record_message(1), 1);
        assert_eq!(tracker.record_message(1), 2);
        assert_eq!(tracker.record_message(1), 3);
        assert_eq!(tracker.count(1), 3);
    }

    #[test]
    fn different_channels_independent() {
        let tracker = SlowmodeTracker::<u64>::new(60);
        tracker.record_message(1);
        tracker.record_message(1);
        tracker.record_message(2);
        assert_eq!(tracker.count(1), 2);
        assert_eq!(tracker.count(2), 1);
    }

    #[test]
    fn try_start_activation_first_time_then_blocked() {
        let tracker = SlowmodeTracker::<u64>::new(60);
        assert!(tracker.try_start_activation(1));
        assert!(!tracker.try_start_activation(1));
        tracker.finish_activation(1);
        assert!(tracker.try_start_activation(1));
    }

    #[test]
    fn tracked_channels_count() {
        let tracker = SlowmodeTracker::<u64>::new(60);
        assert_eq!(tracker.tracked_channels(), 0);
        tracker.record_message(1);
        tracker.record_message(2);
        assert_eq!(tracker.tracked_channels(), 2);
    }

    #[test]
    fn cleanup_removes_old_channels() {
        let tracker = SlowmodeTracker::<u64>::new(1); // window 1s
        tracker.record_message(1);
        tracker
            .counters
            .backdate(&1, std::time::Duration::from_secs(10));
        tracker.cleanup();
        assert_eq!(tracker.tracked_channels(), 0);
    }

    #[test]
    fn should_activate_threshold() {
        let tracker = SlowmodeTracker::<u64>::new(60);
        tracker.record_message(1);
        tracker.record_message(1);
        assert!(!tracker.should_activate(1, 5));
        for _ in 0..3 {
            tracker.record_message(1);
        }
        assert!(tracker.should_activate(1, 5));
        assert!(!tracker.should_activate(2, 5)); // salon vide
    }

    #[test]
    fn reset_clears() {
        let tracker = SlowmodeTracker::<u64>::new(60);
        tracker.record_message(1);
        tracker.record_message(1);
        tracker.reset(1);
        assert_eq!(tracker.count(1), 0);
    }

    #[test]
    fn deactivate_returns_only_quiet_automod_channels() {
        let tracker = SlowmodeTracker::<u64>::new(60);
        let quiet = 1u64;
        let busy = 2u64;
        let manual = 9u64;
        tracker.mark_active(quiet);
        tracker.mark_active(busy);
        tracker.record_message(quiet); // sous seuil/2
        for _ in 0..8 {
            tracker.record_message(busy); // au-dessus de seuil/2
        }
        tracker.record_message(manual); // calme mais PAS mark_active
        let to_deactivate = tracker.channels_to_deactivate(10);
        assert_eq!(to_deactivate, vec![quiet]);
    }

    #[test]
    fn deactivate_empty_tracker() {
        let tracker = SlowmodeTracker::<u64>::new(60);
        assert!(tracker.channels_to_deactivate(10).is_empty());
    }
}
