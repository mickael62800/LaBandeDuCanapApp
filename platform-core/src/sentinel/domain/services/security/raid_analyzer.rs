use std::hash::Hash;
use std::time::{Duration, Instant};

use dashmap::DashMap;

/// Metadata d'un utilisateur qui rejoint le serveur.
#[derive(Clone, Debug)]
pub struct JoinInfo {
    pub username: String,
    pub has_avatar: bool,
    pub account_created_timestamp: i64,
}

/// Tracker des joins récents avec metadata (parallèle au `RaidDetector`).
/// Générique sur la clé `K` (identifiant de serveur fourni par l'adaptateur)
/// pour rester pur — pas de dépendance Discord dans le core.
pub struct RecentJoinsTracker<K: Eq + Hash + Clone> {
    joins: DashMap<K, Vec<(Instant, JoinInfo)>>,
    window: Duration,
}

impl<K: Eq + Hash + Clone> RecentJoinsTracker<K> {
    pub fn new(window_secs: u64) -> Self {
        Self {
            joins: DashMap::new(),
            window: Duration::from_secs(window_secs),
        }
    }

    /// Enregistre un join.
    pub fn record(&self, key: K, info: JoinInfo) {
        let now = Instant::now();
        let mut entry = self.joins.entry(key).or_default();
        let list = entry.value_mut();
        list.retain(|(t, _)| now.duration_since(*t) < self.window);
        list.push((now, info));
    }

    /// Retourne les `JoinInfo` récentes pour une clé.
    pub fn recent(&self, key: K) -> Vec<JoinInfo> {
        let now = Instant::now();
        self.joins
            .get(&key)
            .map(|entry| {
                entry
                    .value()
                    .iter()
                    .filter(|(t, _)| now.duration_since(*t) < self.window)
                    .map(|(_, info)| info.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Reset après traitement raid.
    pub fn reset(&self, key: K) {
        self.joins.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(name: &str) -> JoinInfo {
        JoinInfo {
            username: name.to_string(),
            has_avatar: true,
            account_created_timestamp: 0,
        }
    }

    #[test]
    fn tracker_record_and_recent() {
        let tracker = RecentJoinsTracker::<u64>::new(60);
        tracker.record(1, info("alice"));
        tracker.record(1, info("bob"));
        assert_eq!(tracker.recent(1).len(), 2);
    }

    #[test]
    fn tracker_different_guilds() {
        let tracker = RecentJoinsTracker::<u64>::new(60);
        tracker.record(1, info("a"));
        tracker.record(2, info("b"));
        assert_eq!(tracker.recent(1).len(), 1);
        assert_eq!(tracker.recent(2).len(), 1);
    }

    #[test]
    fn tracker_reset() {
        let tracker = RecentJoinsTracker::<u64>::new(60);
        tracker.record(1, info("a"));
        tracker.reset(1);
        assert_eq!(tracker.recent(1).len(), 0);
    }
}
