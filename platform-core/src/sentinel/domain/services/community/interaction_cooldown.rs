//! Rate limiter per-user pour eviter le spam d'interactions (boutons,
//! commandes). Le `bucket_key` est un identifiant libre qui permet d'avoir
//! plusieurs cooldowns distincts par user (ex: "role_toggle",
//! "parrain_command"). Le mécanisme (check-and-set atomique, purge amortie)
//! vit dans `CooldownMap`.

use crate::sentinel::domain::services::cooldown_map::CooldownMap;

/// Âge maximal utilisé par la purge amortie. Les buckets ont des cooldowns
/// hétérogènes (2 s pour les boutons de rôle, 30 s+ configurables pour le
/// parrainage) : la purge doit couvrir le plus long — 60 s suffisent pour
/// tous les cas.
const PURGE_MAX_AGE_SECS: u64 = 60;

/// Bucket = (user_id, key) -> dernier timestamp de trigger.
pub struct InteractionCooldown {
    map: CooldownMap<(u64, String)>,
}

impl InteractionCooldown {
    pub fn new() -> Self {
        Self {
            map: CooldownMap::new(1000),
        }
    }

    /// Verifie le cooldown. Retourne `Some(remaining_secs)` si le user doit
    /// encore attendre, `None` si l'action est autorisee (et alors enregistre
    /// le nouveau timestamp).
    pub fn check_and_set(&self, user_id: u64, key: &str, cooldown_secs: u64) -> Option<u64> {
        self.map.check_and_set(
            (user_id, key.to_string()),
            cooldown_secs,
            PURGE_MAX_AGE_SECS.max(cooldown_secs),
        )
    }
}

impl Default for InteractionCooldown {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn first_call_allowed() {
        let c = InteractionCooldown::new();
        assert_eq!(c.check_and_set(1, "role", 5), None);
    }

    #[test]
    fn second_call_blocked() {
        let c = InteractionCooldown::new();
        c.check_and_set(1, "role", 5);
        let result = c.check_and_set(1, "role", 5);
        assert!(result.is_some());
        assert!(result.unwrap() <= 5);
    }

    #[test]
    fn different_keys_independent() {
        let c = InteractionCooldown::new();
        c.check_and_set(1, "role", 5);
        // Meme user, clé differente → pas de cooldown
        assert_eq!(c.check_and_set(1, "parrain", 5), None);
    }

    #[test]
    fn different_users_independent() {
        let c = InteractionCooldown::new();
        c.check_and_set(1, "role", 5);
        assert_eq!(c.check_and_set(2, "role", 5), None);
    }

    #[test]
    fn cooldown_expires() {
        let c = InteractionCooldown::new();
        c.check_and_set(1, "role", 1);
        sleep(Duration::from_millis(1100));
        assert_eq!(c.check_and_set(1, "role", 1), None);
    }
}
