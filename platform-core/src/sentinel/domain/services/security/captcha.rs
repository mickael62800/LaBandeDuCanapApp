//! Captcha — logique PURE (génération du challenge + suivi des captchas en
//! attente avec TTL). L'envoi Discord (DM, boutons) reste dans l'adaptateur.
//! `CaptchaPending` est générique sur la clé `K` (l'adaptateur fournit son
//! couple `(GuildId, UserId)`) pour que le core ne connaisse pas Discord.

use std::hash::Hash;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use rand::seq::SliceRandom;
use rand::Rng;

/// Suivi des captchas math en attente. Clé fournie par l'adaptateur
/// (typiquement `(GuildId, UserId)`). Valeur : (index correct, timestamp).
pub struct CaptchaPending<K: Eq + Hash + Copy> {
    pending: DashMap<K, (usize, Instant)>,
    /// Durée de vie d'un captcha en secondes (au-delà l'entrée est invalide).
    ttl_secs: u64,
}

impl<K: Eq + Hash + Copy> Default for CaptchaPending<K> {
    fn default() -> Self {
        Self::with_ttl(600) // 10 minutes par défaut
    }
}

impl<K: Eq + Hash + Copy> CaptchaPending<K> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ttl(ttl_secs: u64) -> Self {
        Self {
            pending: DashMap::new(),
            ttl_secs,
        }
    }

    /// Enregistre un captcha math en attente.
    pub fn store(&self, key: K, correct_index: usize) {
        self.pending.insert(key, (correct_index, Instant::now()));
    }

    /// Vérifie si le bouton pressé est correct. Retourne :
    /// - `Some(true/false)` si un captcha valide existe
    /// - `None` si aucun captcha en attente OU si l'entrée a expiré
    pub fn verify(&self, key: K, pressed_index: usize) -> Option<bool> {
        let entry = self.pending.get(&key)?;
        let (correct, stored_at) = *entry.value();
        drop(entry);
        if stored_at.elapsed() >= Duration::from_secs(self.ttl_secs) {
            // Entrée expirée : on la supprime et on considère qu'il n'y a plus de captcha.
            self.pending.remove(&key);
            return None;
        }
        // USAGE UNIQUE (anti brute-force) : on consomme l'entrée que la réponse
        // soit bonne OU mauvaise. Sinon un self-bot cliquerait les 4 boutons et
        // trouverait le bon en <= 4 essais garantis. Une mauvaise réponse
        // invalide donc le captcha (le user devra en obtenir un nouveau).
        self.pending.remove(&key);
        Some(pressed_index == correct)
    }

    /// Supprime toutes les entrées expirées. Appelé par la task de background.
    pub fn cleanup_expired(&self) {
        let ttl = Duration::from_secs(self.ttl_secs);
        self.pending.retain(|_, (_, ts)| ts.elapsed() < ttl);
    }

    /// Supprime un captcha en attente (après vérification ou timeout).
    pub fn remove(&self, key: K) {
        self.pending.remove(&key);
    }

    /// Indique si un captcha MATH (non expiré) est en attente pour cette clé.
    /// Sert à interdire le bypass du math via le bouton simple.
    pub fn is_pending(&self, key: K) -> bool {
        self.pending
            .get(&key)
            .map(|e| e.value().1.elapsed() < Duration::from_secs(self.ttl_secs))
            .unwrap_or(false)
    }

    /// Retourne les captchas expirés (pour nettoyage).
    pub fn expired(&self, timeout_secs: u64) -> Vec<K> {
        let timeout = Duration::from_secs(timeout_secs);
        let now = Instant::now();
        self.pending
            .iter()
            .filter(|entry| now.duration_since(entry.value().1) >= timeout)
            .map(|entry| *entry.key())
            .collect()
    }
}

/// Génère un challenge mathématique.
/// Retourne (question, index de la bonne réponse, libellés des 4 choix).
pub fn generate_math_challenge() -> (String, usize, Vec<String>) {
    let mut rng = rand::thread_rng();
    let a = rng.gen_range(1..20u32);
    let b = rng.gen_range(1..20u32);
    let correct = a + b;

    let mut choices: Vec<u32> = vec![correct];
    while choices.len() < 4 {
        let wrong = rng.gen_range(2..40u32);
        if !choices.contains(&wrong) {
            choices.push(wrong);
        }
    }

    choices.shuffle(&mut rng);

    let correct_index = choices.iter().position(|&v| v == correct).unwrap();
    let labels: Vec<String> = choices.iter().map(|v| v.to_string()).collect();
    let question = format!("Combien font {} + {} ?", a, b);

    (question, correct_index, labels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_challenge_has_correct_in_choices() {
        for _ in 0..50 {
            let (_, idx, labels) = generate_math_challenge();
            assert_eq!(labels.len(), 4);
            assert!(idx < 4);
            // Les 4 choix sont distincts.
            let mut sorted = labels.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), 4, "choix dupliqués");
        }
    }

    #[test]
    fn verify_correct_and_single_use() {
        let p = CaptchaPending::<(u64, u64)>::with_ttl(600);
        p.store((1, 2), 3);
        assert_eq!(p.verify((1, 2), 3), Some(true));
        // Usage unique : la 2e vérification ne trouve plus rien.
        assert_eq!(p.verify((1, 2), 3), None);
    }

    #[test]
    fn wrong_answer_consumes_entry() {
        let p = CaptchaPending::<(u64, u64)>::with_ttl(600);
        p.store((1, 2), 3);
        assert_eq!(p.verify((1, 2), 0), Some(false));
        assert!(!p.is_pending((1, 2)));
    }

    #[test]
    fn expired_entry_returns_none() {
        let p = CaptchaPending::<(u64, u64)>::with_ttl(0);
        p.store((1, 2), 3);
        assert_eq!(p.verify((1, 2), 3), None);
    }
}
