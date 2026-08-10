//! État de la carte de changement de rôles « vivante » (anti-spam).
//!
//! Une seule carte par membre, active pendant une fenêtre glissante ; elle
//! cumule l'HISTORIQUE chronologique des mouvements et expire sans activité.
//! Ici : l'état (map fenêtrée) et les règles (bornes de fenêtre, troncature
//! d'affichage). L'édition du message Discord et l'embed restent dans le bot.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::Instant;

/// `(true = ajouté, role_id)` dans l'ordre chronologique.
pub type RoleMovement = (bool, String);

struct CardState {
    channel_id: u64,
    message_id: u64,
    movements: Vec<RoleMovement>,
    expires_at: Instant,
}

/// Tracker générique — la clé est fournie par l'adaptateur (ex.
/// `(guild_id, user_id)` en String).
#[derive(Default)]
pub struct RoleCardTracker<K: Eq + Hash + Clone> {
    inner: Mutex<HashMap<K, CardState>>,
}

impl<K: Eq + Hash + Clone> RoleCardTracker<K> {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Purge les cartes expirées puis retourne un snapshot de la carte active
    /// pour la clé : `(channel_id, message_id, movements)`.
    pub fn active(&self, key: &K, now: Instant) -> Option<(u64, u64, Vec<RoleMovement>)> {
        let mut map = self.inner.lock().unwrap();
        map.retain(|_, c| c.expires_at > now);
        map.get(key)
            .map(|c| (c.channel_id, c.message_id, c.movements.clone()))
    }

    /// Met à jour la carte existante (historique cumulé + renouvellement de
    /// la fenêtre). No-op si la carte a expiré entre-temps.
    pub fn update(&self, key: &K, movements: Vec<RoleMovement>, expires_at: Instant) {
        let mut map = self.inner.lock().unwrap();
        if let Some(c) = map.get_mut(key) {
            c.movements = movements;
            c.expires_at = expires_at;
        }
    }

    /// Enregistre une nouvelle carte (après le post Discord réussi).
    pub fn insert(
        &self,
        key: K,
        channel_id: u64,
        message_id: u64,
        movements: Vec<RoleMovement>,
        expires_at: Instant,
    ) {
        self.inner.lock().unwrap().insert(
            key,
            CardState {
                channel_id,
                message_id,
                movements,
                expires_at,
            },
        );
    }
}

/// Fenêtre d'activité de la carte : configurée en secondes, bornée 10s..1h,
/// défaut 2 min.
pub fn clamp_role_log_window(configured: Option<u64>) -> u64 {
    configured.unwrap_or(120).clamp(10, 3600)
}

/// Troncature d'affichage : retourne `(nb_masqués, tranche visible)` — les
/// `max_lines` mouvements les plus récents, ordre chronologique conservé.
pub fn visible_movements(movements: &[RoleMovement], max_lines: usize) -> (usize, &[RoleMovement]) {
    let start = movements.len().saturating_sub(max_lines);
    (start, &movements[start..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn mv(n: usize) -> Vec<RoleMovement> {
        (0..n).map(|i| (i % 2 == 0, i.to_string())).collect()
    }

    #[test]
    fn active_none_initially_then_snapshot() {
        let t = RoleCardTracker::<u64>::new();
        let now = Instant::now();
        assert!(t.active(&1, now).is_none());
        t.insert(1, 10, 20, mv(2), now + Duration::from_secs(60));
        let (chan, msg, m) = t.active(&1, now).unwrap();
        assert_eq!((chan, msg, m.len()), (10, 20, 2));
    }

    #[test]
    fn expired_card_is_purged() {
        let t = RoleCardTracker::<u64>::new();
        let now = Instant::now();
        t.insert(1, 10, 20, mv(1), now + Duration::from_secs(1));
        assert!(t.active(&1, now + Duration::from_secs(2)).is_none());
    }

    #[test]
    fn update_renews_window_and_history() {
        let t = RoleCardTracker::<u64>::new();
        let now = Instant::now();
        t.insert(1, 10, 20, mv(1), now + Duration::from_secs(5));
        t.update(&1, mv(4), now + Duration::from_secs(120));
        let (_, _, m) = t.active(&1, now + Duration::from_secs(60)).unwrap();
        assert_eq!(m.len(), 4);
    }

    #[test]
    fn window_bounds() {
        assert_eq!(clamp_role_log_window(None), 120);
        assert_eq!(clamp_role_log_window(Some(1)), 10);
        assert_eq!(clamp_role_log_window(Some(999_999)), 3600);
    }

    #[test]
    fn visible_truncation_keeps_most_recent() {
        let m = mv(30);
        let (hidden, shown) = visible_movements(&m, 25);
        assert_eq!(hidden, 5);
        assert_eq!(shown.len(), 25);
        assert_eq!(shown[0].1, "5"); // les 5 plus anciens masqués
        let (hidden, shown) = visible_movements(&m[..3], 25);
        assert_eq!(hidden, 0);
        assert_eq!(shown.len(), 3);
    }
}
