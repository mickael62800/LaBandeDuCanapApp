//! Fenêtre glissante de timestamps par clé — le stockage commun des trackers
//! (raid, flood, slowmode adaptatif). La fenêtre est passée à CHAQUE appel
//! (certains trackers la reconfigurent à chaud via un `AtomicU64`) ; la
//! DÉCISION (seuils, activation) reste dans chaque tracker.
//!
//! Politique de purge UNIQUE : `prune_if_larger(max_entries, max_age)` retire
//! les clés sans activité depuis `max_age` dès que la map dépasse
//! `max_entries`. À appeler depuis les chemins `record` (jamais en tenant le
//! lock d'une entry : `retain` verrouille tous les shards).

use std::hash::Hash;
use std::time::{Duration, Instant};

use dashmap::DashMap;

pub struct SlidingWindow<K: Eq + Hash + Clone> {
    map: DashMap<K, Vec<Instant>>,
}

impl<K: Eq + Hash + Clone> SlidingWindow<K> {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    /// Enregistre un évènement et retourne le nombre d'évènements dans la
    /// fenêtre (celui qui vient d'être ajouté compris).
    pub fn record(&self, key: K, window: Duration) -> usize {
        let now = Instant::now();
        let mut entry = self.map.entry(key).or_default();
        let timestamps = entry.value_mut();
        timestamps.retain(|t| now.duration_since(*t) < window);
        timestamps.push(now);
        timestamps.len()
    }

    /// Nombre d'évènements dans la fenêtre pour une clé (lecture pure).
    pub fn count(&self, key: &K, window: Duration) -> usize {
        let now = Instant::now();
        self.map
            .get(key)
            .map(|entry| {
                entry
                    .value()
                    .iter()
                    .filter(|t| now.duration_since(**t) < window)
                    .count()
            })
            .unwrap_or(0)
    }

    /// Supprime la fenêtre d'une clé.
    pub fn clear(&self, key: &K) {
        self.map.remove(key);
    }

    /// Nombre de clés actuellement suivies.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Purge les clés sans activité depuis `max_age`.
    pub fn prune(&self, max_age: Duration) {
        let now = Instant::now();
        self.map.retain(|_, ts| {
            ts.last()
                .map(|t| now.duration_since(*t) < max_age)
                .unwrap_or(false)
        });
    }

    /// Purge uniquement quand la map dépasse `max_entries` (amorti : évite de
    /// verrouiller tous les shards à chaque record).
    pub fn prune_if_larger(&self, max_entries: usize, max_age: Duration) {
        if self.map.len() > max_entries {
            self.prune(max_age);
        }
    }

    /// Test uniquement : vieillit tous les timestamps d'une clé de `age`.
    #[cfg(test)]
    pub fn backdate(&self, key: &K, age: Duration) {
        if let Some(mut e) = self.map.get_mut(key) {
            for t in e.value_mut() {
                *t -= age;
            }
        }
    }
}

impl<K: Eq + Hash + Clone> Default for SlidingWindow<K> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: Duration = Duration::from_secs(60);

    #[test]
    fn record_counts_within_window() {
        let w = SlidingWindow::<u64>::new();
        assert_eq!(w.record(1, W), 1);
        assert_eq!(w.record(1, W), 2);
        assert_eq!(w.count(&1, W), 2);
        assert_eq!(w.count(&2, W), 0);
    }

    #[test]
    fn expired_events_dropped_on_record() {
        let w = SlidingWindow::<u64>::new();
        w.record(1, W);
        w.backdate(&1, Duration::from_secs(120));
        // L'évènement expiré est purgé par le retain du record suivant.
        assert_eq!(w.record(1, W), 1);
    }

    #[test]
    fn clear_resets_key() {
        let w = SlidingWindow::<u64>::new();
        w.record(1, W);
        w.clear(&1);
        assert_eq!(w.count(&1, W), 0);
        assert!(w.is_empty());
    }

    #[test]
    fn prune_removes_stale_keys() {
        let w = SlidingWindow::<u64>::new();
        w.record(1, W);
        w.record(2, W);
        w.backdate(&1, Duration::from_secs(300));
        w.prune(Duration::from_secs(120));
        assert_eq!(w.len(), 1);
        assert_eq!(w.count(&2, W), 1);
    }

    #[test]
    fn prune_if_larger_is_amortized() {
        let w = SlidingWindow::<u64>::new();
        w.record(1, W);
        w.backdate(&1, Duration::from_secs(300));
        // Sous le seuil : pas de purge.
        w.prune_if_larger(10, Duration::from_secs(120));
        assert_eq!(w.len(), 1);
        // Au-dessus du seuil : purge.
        w.prune_if_larger(0, Duration::from_secs(120));
        assert_eq!(w.len(), 0);
    }
}
