use std::hash::Hash;
use std::time::Instant;

use dashmap::DashMap;

/// Suivi des utilisateurs AFK en vocal (mute + sourd). Générique sur la clé
/// `U` (l'adaptateur fournit son type d'identifiant, ex. `UserId`).
pub struct AfkTracker<U: Eq + Hash + Copy> {
    map: DashMap<U, Instant>,
}

impl<U: Eq + Hash + Copy> AfkTracker<U> {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    /// Marque un utilisateur comme AFK (mute + sourd).
    /// Ne met a jour que si pas deja traque.
    pub fn mark_afk(&self, user_id: U) {
        self.map.entry(user_id).or_insert_with(Instant::now);
    }

    /// Retire le marquage AFK (unmute, undeaf, ou leave).
    pub fn clear(&self, user_id: U) {
        self.map.remove(&user_id);
    }

    /// Retourne l'instant ou l'utilisateur est devenu AFK.
    pub fn get_afk_since(&self, user_id: U) -> Option<Instant> {
        self.map.get(&user_id).map(|entry| *entry.value())
    }

    /// Retourne tous les utilisateurs AFK avec leur instant de debut.
    pub fn afk_users(&self) -> Vec<(U, Instant)> {
        self.map
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect()
    }
}

impl<U: Eq + Hash + Copy> Default for AfkTracker<U> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Tracker = AfkTracker<u64>;

    #[test]
    fn test_not_afk_initially() {
        let tracker = Tracker::new();
        assert!(tracker.get_afk_since(1).is_none());
    }

    #[test]
    fn test_mark_afk() {
        let tracker = Tracker::new();
        tracker.mark_afk(1);
        assert!(tracker.get_afk_since(1).is_some());
    }

    #[test]
    fn test_clear_afk() {
        let tracker = Tracker::new();
        tracker.mark_afk(1);
        tracker.clear(1);
        assert!(tracker.get_afk_since(1).is_none());
    }

    #[test]
    fn test_mark_does_not_reset_existing() {
        let tracker = Tracker::new();
        tracker.mark_afk(1);
        let first = tracker.get_afk_since(1).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        tracker.mark_afk(1);
        let second = tracker.get_afk_since(1).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn test_different_users_independent() {
        let tracker = Tracker::new();
        tracker.mark_afk(1);
        assert!(tracker.get_afk_since(1).is_some());
        assert!(tracker.get_afk_since(2).is_none());
    }

    #[test]
    fn test_afk_users_returns_all() {
        let tracker = Tracker::new();
        tracker.mark_afk(1);
        tracker.mark_afk(2);
        tracker.mark_afk(3);
        let users = tracker.afk_users();
        assert_eq!(users.len(), 3);
    }

    #[test]
    fn test_afk_users_empty() {
        let tracker = Tracker::new();
        assert!(tracker.afk_users().is_empty());
    }
}
