//! Cooldown par clé avec check-and-set atomique — le mécanisme commun de
//! `voice::CooldownTracker` et `community::InteractionCooldown`.
//!
//! Atomicité : check-then-set en une seule opération verrouillée via l'API
//! `entry` de DashMap. Le shard de la clé reste verrouillé entre la lecture du
//! timestamp et l'écriture, donc deux évènements concurrents sur la même clé
//! ne peuvent pas passer tous les deux (TOCTOU corrigé).
//!
//! Purge UNIQUE : amortie dans `check_and_set`, quand la map dépasse
//! `max_entries`, on retire les entrées plus vieilles que `purge_max_age_secs`.
//! L'âge de purge est un paramètre explicite de l'appel (et non le cooldown de
//! l'appel courant) : une map multi-buckets aux cooldowns hétérogènes (ex.
//! `InteractionCooldown`) ne doit pas laisser un appel à cooldown court
//! évincer les cooldowns longs encore actifs.
//! Fait AVANT le `entry` (retain verrouille tous les shards, l'appeler en
//! tenant le lock d'une entry risquerait un deadlock).

use std::hash::Hash;
use std::time::Instant;

use dashmap::DashMap;

pub struct CooldownMap<K: Eq + Hash> {
    map: DashMap<K, Instant>,
    max_entries: usize,
}

impl<K: Eq + Hash> CooldownMap<K> {
    pub fn new(max_entries: usize) -> Self {
        Self {
            map: DashMap::new(),
            max_entries,
        }
    }

    /// Vérifie ET pose le cooldown de manière atomique. Retourne
    /// `Some(remaining_secs)` si la clé est encore en cooldown (rien n'est
    /// écrit), `None` si l'action est autorisée (le timestamp est alors
    /// enregistré). `purge_max_age_secs` borne l'âge au-delà duquel une entrée
    /// est évincée lors de la purge amortie — il doit couvrir le plus long
    /// cooldown utilisé sur cette map.
    pub fn check_and_set(
        &self,
        key: K,
        cooldown_secs: u64,
        purge_max_age_secs: u64,
    ) -> Option<u64> {
        let now = Instant::now();

        if self.map.len() > self.max_entries {
            self.map
                .retain(|_, ts| ts.elapsed().as_secs() < purge_max_age_secs);
        }

        use dashmap::mapref::entry::Entry;
        match self.map.entry(key) {
            Entry::Occupied(mut e) => {
                let elapsed = e.get().elapsed().as_secs();
                if elapsed < cooldown_secs {
                    return Some(cooldown_secs - elapsed);
                }
                e.insert(now);
                None
            }
            Entry::Vacant(e) => {
                e.insert(now);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_call_allowed_second_blocked() {
        let c = CooldownMap::<u64>::new(1000);
        assert_eq!(c.check_and_set(1, 5, 60), None);
        let remaining = c.check_and_set(1, 5, 60);
        assert!(remaining.is_some());
        assert!(remaining.unwrap() <= 5);
    }

    #[test]
    fn keys_independent() {
        let c = CooldownMap::<u64>::new(1000);
        c.check_and_set(1, 5, 60);
        assert_eq!(c.check_and_set(2, 5, 60), None);
    }

    #[test]
    fn cooldown_expires() {
        let c = CooldownMap::<u64>::new(1000);
        c.check_and_set(1, 1, 60);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert_eq!(c.check_and_set(1, 1, 60), None);
    }
}
