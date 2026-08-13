use std::hash::Hash;

use dashmap::DashMap;

/// Message cache pour retrouver le contenu des messages supprimes.
#[derive(Clone, Debug)]
pub struct CachedMessage {
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub channel_id: String,
    /// True si l'auteur est un bot : permet d'exclure ses editions/suppressions
    /// des logs Discord.
    pub is_bot: bool,
}

/// Cache LRU simplifie pour les messages par guild. Générique sur les clés
/// `G` (serveur) et `M` (message) — `M: Ord` car l'éviction trie par ID
/// croissant (snowflake Discord = plus petit ID = plus vieux message).
pub struct MessageCache<G: Eq + Hash + Copy, M: Eq + Hash + Copy + Ord> {
    cache: DashMap<(G, M), CachedMessage>,
    max_per_guild: usize,
    /// Compteur de messages par guild pour savoir quand evicter.
    counts: DashMap<G, usize>,
}

impl<G: Eq + Hash + Copy, M: Eq + Hash + Copy + Ord> MessageCache<G, M> {
    pub fn new(max_per_guild: usize) -> Self {
        Self {
            cache: DashMap::new(),
            max_per_guild,
            counts: DashMap::new(),
        }
    }

    /// Stocke un message dans le cache.
    pub fn store(&self, guild_id: G, message_id: M, cached: CachedMessage) {
        // Eviction si on depasse la limite : trier par MessageId croissant
        // (snowflake Discord = plus petit ID = plus vieux message).
        let current_count = self.counts.get(&guild_id).map(|c| *c).unwrap_or(0);
        if current_count >= self.max_per_guild {
            let evict_count = self.max_per_guild / 10;
            let mut guild_keys: Vec<(G, M)> = self
                .cache
                .iter()
                .filter(|e| e.key().0 == guild_id)
                .map(|e| *e.key())
                .collect();
            guild_keys.sort_by_key(|k| k.1);
            let to_remove = &guild_keys[..evict_count.min(guild_keys.len())];

            for key in to_remove {
                self.cache.remove(key);
            }
            if let Some(mut count) = self.counts.get_mut(&guild_id) {
                *count = count.saturating_sub(to_remove.len());
            }
        }

        // On n'incremente que sur une insertion REELLE : re-`store` d'un meme
        // message_id (edition, ou re-cache) REMPLACE l'entree sans en ajouter.
        // Sinon le compteur derive au-dessus de cache.len() -> eviction
        // prematuree de vrais messages (contenu d'audit perdu).
        let is_new = self.cache.insert((guild_id, message_id), cached).is_none();
        let mut count = self.counts.entry(guild_id).or_insert(0);
        if is_new {
            *count += 1;
        }

        // Garde de securite globale : empecher le cache de depasser 2x la limite
        if *count > self.max_per_guild * 2 {
            let excess = *count - self.max_per_guild;
            let mut guild_keys: Vec<(G, M)> = self
                .cache
                .iter()
                .filter(|e| e.key().0 == guild_id)
                .map(|e| *e.key())
                .collect();
            guild_keys.sort_by_key(|k| k.1);
            let to_remove = &guild_keys[..excess.min(guild_keys.len())];
            for key in to_remove {
                self.cache.remove(key);
            }
            *count = count.saturating_sub(to_remove.len());
        }
    }

    /// Recupere un message du cache.
    pub fn get(&self, guild_id: G, message_id: M) -> Option<CachedMessage> {
        self.cache.get(&(guild_id, message_id)).map(|e| e.clone())
    }

    /// Supprime un message du cache.
    pub fn remove(&self, guild_id: G, message_id: M) -> Option<CachedMessage> {
        let removed = self.cache.remove(&(guild_id, message_id));
        if removed.is_some() {
            if let Some(mut count) = self.counts.get_mut(&guild_id) {
                *count = count.saturating_sub(1);
            }
        }
        removed.map(|(_, v)| v)
    }

    /// Nombre de messages en cache pour un guild.
    pub fn count(&self, guild_id: G) -> usize {
        self.counts.get(&guild_id).map(|c| *c).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Cache = MessageCache<u64, u64>;

    fn make_cached(content: &str) -> CachedMessage {
        CachedMessage {
            author_id: "123".to_string(),
            author_name: "Alice".to_string(),
            content: content.to_string(),
            channel_id: "456".to_string(),
            is_bot: false,
        }
    }

    #[test]
    fn store_and_get() {
        let cache = Cache::new(100);
        cache.store(1, 42, make_cached("hello"));

        let result = cache.get(1, 42);
        assert!(result.is_some());
        assert_eq!(result.unwrap().content, "hello");
    }

    #[test]
    fn get_missing() {
        let cache = Cache::new(100);
        assert!(cache.get(1, 42).is_none());
    }

    #[test]
    fn remove_returns_value() {
        let cache = Cache::new(100);
        cache.store(1, 42, make_cached("hello"));
        let removed = cache.remove(1, 42);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().content, "hello");

        // Plus dans le cache
        assert!(cache.get(1, 42).is_none());
    }

    #[test]
    fn remove_missing_returns_none() {
        let cache = Cache::new(100);
        assert!(cache.remove(1, 1).is_none());
    }

    #[test]
    fn count_tracks_correctly() {
        let cache = Cache::new(100);
        assert_eq!(cache.count(1), 0);

        cache.store(1, 1, make_cached("a"));
        cache.store(1, 2, make_cached("b"));
        assert_eq!(cache.count(1), 2);

        cache.remove(1, 1);
        assert_eq!(cache.count(1), 1);
    }

    #[test]
    fn different_guilds_independent() {
        let cache = Cache::new(100);
        cache.store(1, 42, make_cached("guild A"));
        cache.store(2, 42, make_cached("guild B"));

        assert_eq!(cache.get(1, 42).unwrap().content, "guild A");
        assert_eq!(cache.get(2, 42).unwrap().content, "guild B");
    }

    #[test]
    fn eviction_on_overflow() {
        let cache = Cache::new(10);

        // Remplir le cache
        for i in 1..=10 {
            cache.store(1, i, make_cached(&format!("msg {}", i)));
        }
        assert_eq!(cache.count(1), 10);

        // Ajouter un 11e devrait declencher l'eviction
        cache.store(1, 11, make_cached("msg 11"));

        // Le count doit etre <= max (10% evictes + 1 ajoute = 10 - 1 + 1 = 10)
        assert!(cache.count(1) <= 10);

        // Le dernier message est present
        assert!(cache.get(1, 11).is_some());
    }

    #[test]
    fn eviction_removes_oldest_ids_first() {
        let cache = Cache::new(10);
        for i in 1..=10 {
            cache.store(1, i, make_cached("m"));
        }
        cache.store(1, 11, make_cached("m"));

        // L'éviction retire les plus petits IDs (plus vieux snowflakes).
        assert!(cache.get(1, 1).is_none());
        assert!(cache.get(1, 10).is_some());
    }

    #[test]
    fn restore_same_id_does_not_drift_count() {
        let cache = Cache::new(100);
        cache.store(1, 42, make_cached("v1"));
        cache.store(1, 42, make_cached("v2")); // edition : remplace, pas d'ajout
        assert_eq!(cache.count(1), 1);
        assert_eq!(cache.get(1, 42).unwrap().content, "v2");
    }
}
