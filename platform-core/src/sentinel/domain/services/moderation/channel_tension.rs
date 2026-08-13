//! Systeme de "tension de salon" : somme glissante des scores IA des N
//! derniers messages d'un salon. Si la somme totale depasse un seuil,
//! declenche une action Warn/Delete/Mute en plus de l'analyse individuelle.
//!
//! Buffer in-memory, thread-safe, pas persistant (on reinitialise si le
//! bot restart — c'est OK, les messages toxiques passes sont deja traites).

use crate::sentinel::domain::entities::system::discord_ids::MessageId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;

/// Entry dans le buffer glissant : score IA + auteur + message_id pour
/// pouvoir agir sur le dernier speaker si un seuil est franchi.
#[derive(Debug, Clone)]
pub struct TensionEntry {
    pub score: f64,
    pub user_id: UserId,
    pub message_id: MessageId,
    pub timestamp_ms: i64,
}

/// Action declenchee par le calcul de tension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensionAction {
    None,
    Warn,
    Delete,
    Mute,
}

/// Buffer glissant par (guild_id, channel_id). Thread-safe via Mutex.
/// Pas de dependance externe (evite d'ajouter `dashmap` au Cargo.toml).
///
/// Le buffer se vide naturellement par LRU (taille fixe). Pas de TTL.
pub struct ChannelTensionBuffer {
    inner: Mutex<HashMap<(String, String), VecDeque<TensionEntry>>>,
}

impl Default for ChannelTensionBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelTensionBuffer {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Ajoute une entree dans le buffer du salon et retourne la somme
    /// glissante des scores. Si le buffer depasse `buffer_size`, pop le
    /// plus ancien.
    pub fn push_and_sum(
        &self,
        guild_id: &str,
        channel_id: &str,
        entry: TensionEntry,
        buffer_size: usize,
    ) -> f64 {
        let mut guard = self.inner.lock().expect("channel tension mutex poisoned");
        let key = (guild_id.to_string(), channel_id.to_string());
        let buf = guard.entry(key).or_default();
        buf.push_back(entry);
        // Garantit qu'on ne depasse jamais buffer_size (gere aussi le cas ou
        // buffer_size vient d'etre reduit via la config).
        while buf.len() > buffer_size.max(1) {
            buf.pop_front();
        }
        buf.iter().map(|e| e.score).sum()
    }

    /// Retourne la somme courante sans rien modifier. Utile pour les tests.
    pub fn current_sum(&self, guild_id: &str, channel_id: &str) -> f64 {
        let guard = self.inner.lock().expect("channel tension mutex poisoned");
        guard
            .get(&(guild_id.to_string(), channel_id.to_string()))
            .map(|buf| buf.iter().map(|e| e.score).sum())
            .unwrap_or(0.0)
    }

    /// Vide le buffer d'un salon (apres declenchement d'une action par
    /// exemple, pour eviter de re-declencher au message suivant).
    pub fn clear_channel(&self, guild_id: &str, channel_id: &str) {
        let mut guard = self.inner.lock().expect("channel tension mutex poisoned");
        guard.remove(&(guild_id.to_string(), channel_id.to_string()));
    }

    /// Determine l'action selon les trois seuils. Un seuil a 0.0 est
    /// considere comme desactive (respecte le principe : valeur non
    /// configuree → pas de declenchement sur ce palier).
    pub fn decide_action(
        total: f64,
        threshold_warn: f64,
        threshold_delete: f64,
        threshold_mute: f64,
    ) -> TensionAction {
        if threshold_mute > 0.0 && total >= threshold_mute {
            TensionAction::Mute
        } else if threshold_delete > 0.0 && total >= threshold_delete {
            TensionAction::Delete
        } else if threshold_warn > 0.0 && total >= threshold_warn {
            TensionAction::Warn
        } else {
            TensionAction::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(score: f64) -> TensionEntry {
        TensionEntry {
            score,
            user_id: UserId::new("u1"),
            message_id: MessageId::new("m1"),
            timestamp_ms: 0,
        }
    }

    #[test]
    fn push_and_sum_adds_entry() {
        let buf = ChannelTensionBuffer::new();
        let s = buf.push_and_sum("g", "c", entry(1.5), 5);
        assert!((s - 1.5).abs() < 1e-9);
    }

    #[test]
    fn push_and_sum_accumulates() {
        let buf = ChannelTensionBuffer::new();
        buf.push_and_sum("g", "c", entry(1.0), 5);
        buf.push_and_sum("g", "c", entry(2.0), 5);
        let s = buf.push_and_sum("g", "c", entry(3.0), 5);
        assert!((s - 6.0).abs() < 1e-9);
    }

    #[test]
    fn push_and_sum_pops_oldest_when_over_buffer() {
        let buf = ChannelTensionBuffer::new();
        for _ in 0..5 {
            buf.push_and_sum("g", "c", entry(1.0), 5);
        }
        // Buffer rempli avec 5x1.0 = 5.0, ajouter 6e devrait pop la 1ere
        let s = buf.push_and_sum("g", "c", entry(2.0), 5);
        // 4x1.0 + 2.0 = 6.0
        assert!((s - 6.0).abs() < 1e-9, "got {}", s);
    }

    #[test]
    fn different_channels_have_separate_buffers() {
        let buf = ChannelTensionBuffer::new();
        buf.push_and_sum("g", "c1", entry(5.0), 5);
        let s2 = buf.push_and_sum("g", "c2", entry(1.0), 5);
        assert!((s2 - 1.0).abs() < 1e-9);
        assert!((buf.current_sum("g", "c1") - 5.0).abs() < 1e-9);
    }

    #[test]
    fn different_guilds_have_separate_buffers() {
        let buf = ChannelTensionBuffer::new();
        buf.push_and_sum("g1", "c", entry(5.0), 5);
        let s2 = buf.push_and_sum("g2", "c", entry(1.0), 5);
        assert!((s2 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn clear_channel_resets_buffer() {
        let buf = ChannelTensionBuffer::new();
        buf.push_and_sum("g", "c", entry(3.0), 5);
        buf.clear_channel("g", "c");
        assert!((buf.current_sum("g", "c") - 0.0).abs() < 1e-9);
    }

    #[test]
    fn decide_action_returns_mute_when_above_all_thresholds() {
        let a = ChannelTensionBuffer::decide_action(10.0, 3.0, 5.0, 7.0);
        assert_eq!(a, TensionAction::Mute);
    }

    #[test]
    fn decide_action_returns_delete_when_above_delete_below_mute() {
        let a = ChannelTensionBuffer::decide_action(6.0, 3.0, 5.0, 7.0);
        assert_eq!(a, TensionAction::Delete);
    }

    #[test]
    fn decide_action_returns_warn_when_only_warn_threshold_crossed() {
        let a = ChannelTensionBuffer::decide_action(3.5, 3.0, 5.0, 7.0);
        assert_eq!(a, TensionAction::Warn);
    }

    #[test]
    fn decide_action_returns_none_when_below_all() {
        let a = ChannelTensionBuffer::decide_action(1.0, 3.0, 5.0, 7.0);
        assert_eq!(a, TensionAction::None);
    }

    #[test]
    fn decide_action_zero_threshold_disables_level() {
        // Warn=0 (disabled), delete=5, mute=7 : un total de 4.0 ne doit
        // RIEN declencher (warn est disable, delete pas atteint).
        let a = ChannelTensionBuffer::decide_action(4.0, 0.0, 5.0, 7.0);
        assert_eq!(a, TensionAction::None);
    }

    #[test]
    fn decide_action_exact_threshold_boundary() {
        let a = ChannelTensionBuffer::decide_action(5.0, 3.0, 5.0, 7.0);
        assert_eq!(a, TensionAction::Delete);
    }
}
