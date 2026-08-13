use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::sentinel::domain::services::cooldown_map::CooldownMap;

const DEFAULT_COOLDOWN_SECS: u64 = 5;

/// Cooldown par utilisateur avec check-and-set atomique. Générique sur la clé
/// `U` (l'adaptateur fournit son type d'identifiant, ex. `UserId`). Le
/// mécanisme (atomicité, purge) vit dans `CooldownMap` ; ici on ne garde que
/// la valeur de cooldown reconfigurable à chaud.
pub struct CooldownTracker<U: Eq + Hash> {
    map: CooldownMap<U>,
    cooldown_secs: AtomicU64,
}

impl<U: Eq + Hash> CooldownTracker<U> {
    pub fn new() -> Self {
        Self {
            map: CooldownMap::new(500),
            cooldown_secs: AtomicU64::new(DEFAULT_COOLDOWN_SECS),
        }
    }

    /// Met a jour le cooldown depuis la config API.
    pub fn set_cooldown_secs(&self, secs: u64) {
        self.cooldown_secs.store(secs, Ordering::Relaxed);
    }

    /// Verifie ET pose le cooldown de maniere atomique. Retourne
    /// `Some(remaining_secs)` si l'utilisateur est encore en cooldown,
    /// `None` si l'action est autorisee.
    pub fn check_and_set(&self, user_id: U) -> Option<u64> {
        let cd = self.cooldown_secs.load(Ordering::Relaxed);
        // Cooldown unique sur toute la map : l'âge de purge est le cooldown
        // lui-même, comme avant l'extraction vers `CooldownMap`.
        self.map.check_and_set(user_id, cd, cd)
    }
}

impl<U: Eq + Hash> Default for CooldownTracker<U> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Tracker = CooldownTracker<u64>;

    #[test]
    fn test_no_cooldown_initially() {
        let tracker = Tracker::new();
        assert!(tracker.check_and_set(1).is_none());
    }

    #[test]
    fn test_cooldown_after_set() {
        let tracker = Tracker::new();
        // 1er appel : autorise + pose le timestamp.
        assert!(tracker.check_and_set(1).is_none());
        // 2e appel immediat : encore en cooldown.
        let remaining = tracker.check_and_set(1);
        assert!(remaining.is_some());
        assert!(remaining.unwrap() <= DEFAULT_COOLDOWN_SECS);
    }

    #[test]
    fn test_different_users_independent() {
        let tracker = Tracker::new();
        assert!(tracker.check_and_set(1).is_none());
        // Meme user : bloque. Autre user : autorise.
        assert!(tracker.check_and_set(1).is_some());
        assert!(tracker.check_and_set(2).is_none());
    }
}
