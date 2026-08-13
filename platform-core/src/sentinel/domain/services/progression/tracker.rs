use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

/// Clé unique (guild_id, user_id).
type UserKey = (u64, u64);

/// Statistiques en mémoire pour un utilisateur.
#[derive(Debug, Clone, Default)]
pub struct UserStats {
    pub message_count: u64,
    pub voice_seconds: u64,
}

/// Suivi de l'entrée en vocal d'un utilisateur.
///
/// Pour empecher le farming d'XP en AFK (self_mute + self_deaf), on
/// accumule les secondes `creditees` seulement quand l'utilisateur est
/// *actif*. Les transitions AFK/actif sont pilotees par `set_afk`.
///
/// - `active_since` : timestamp du dernier passage en "actif" (None si AFK).
/// - `credited_seconds` : temps actif cumule avant la fenetre courante.
#[derive(Debug, Clone)]
struct VoiceSession {
    active_since: Option<i64>,
    credited_seconds: u64,
    /// ID du channel vocal courant. Sert a appliquer le multiplicateur
    /// XP du salon (xp_channel_multipliers) au moment du credit.
    channel_id: u64,
}

/// Tracker in-memory pour les messages et le temps vocal.
#[derive(Debug, Clone)]
pub struct StatsTracker {
    stats: Arc<RwLock<HashMap<UserKey, UserStats>>>,
    voice_sessions: Arc<RwLock<HashMap<UserKey, VoiceSession>>>,
}

impl Default for StatsTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl StatsTracker {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(HashMap::new())),
            voice_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Incrémente le compteur de messages d'un utilisateur.
    pub async fn record_message(&self, guild_id: u64, user_id: u64) {
        let mut stats = self.stats.write().await;
        stats.entry((guild_id, user_id)).or_default().message_count += 1;
    }

    /// Enregistre l'entrée d'un utilisateur dans un salon vocal.
    /// `is_afk` = true si l'utilisateur arrive deja self_mute + self_deaf :
    /// dans ce cas la session demarre en pause et aucun temps n'est compte
    /// tant qu'il n'est pas sorti de l'AFK.
    pub async fn voice_join(&self, guild_id: u64, user_id: u64, channel_id: u64, is_afk: bool) {
        let mut sessions = self.voice_sessions.write().await;
        let active_since = if is_afk {
            None
        } else {
            Some(Utc::now().timestamp())
        };
        sessions.insert(
            (guild_id, user_id),
            VoiceSession {
                active_since,
                credited_seconds: 0,
                channel_id,
            },
        );
    }

    /// Met a jour le channel courant d'une session existante (ex: l'user
    /// change de salon vocal sans quitter le serveur). Permet d'appliquer
    /// le bon multiplicateur au prochain tick / voice_leave.
    pub async fn set_voice_channel(&self, guild_id: u64, user_id: u64, channel_id: u64) {
        let mut sessions = self.voice_sessions.write().await;
        if let Some(s) = sessions.get_mut(&(guild_id, user_id)) {
            // Si l'utilisateur etait actif dans l'ancien channel, on credite
            // le tail dans `credited_seconds` AVANT de changer pour appliquer
            // le bon multiplicateur a l'XP courante.
            if let Some(since) = s.active_since {
                let now = Utc::now().timestamp();
                let delta = (now - since).max(0) as u64;
                s.credited_seconds = s.credited_seconds.saturating_add(delta);
                s.active_since = Some(now);
            }
            s.channel_id = channel_id;
        }
    }

    /// Met a jour l'etat AFK d'un utilisateur encore en vocal. Fait la
    /// bascule actif<->AFK en creditant la fenetre active qui se termine.
    /// No-op si l'utilisateur n'est pas dans une session.
    pub async fn set_voice_afk(&self, guild_id: u64, user_id: u64, is_afk: bool) {
        let mut sessions = self.voice_sessions.write().await;
        let Some(session) = sessions.get_mut(&(guild_id, user_id)) else {
            return;
        };
        let now = Utc::now().timestamp();
        match (session.active_since, is_afk) {
            (Some(since), true) => {
                // Etait actif, passe AFK : crediter la fenetre.
                let delta = (now - since).max(0) as u64;
                session.credited_seconds = session.credited_seconds.saturating_add(delta);
                session.active_since = None;
            }
            (None, false) => {
                // Etait AFK, revient actif : reprendre le compteur.
                session.active_since = Some(now);
            }
            _ => {} // pas de transition
        }
    }

    /// Crédite le temps actif des sessions en cours sans attendre que
    /// l'utilisateur quitte le vocal. Permet aux users qui restent
    /// connectes 24/7 d'accumuler de l'XP regulierement. Retourne
    /// `Vec<(guild_id, user_id, seconds_credited)>` pour que l'appelant
    /// puisse enregistrer l'XP en DB. `active_since` est remis a now
    /// pour eviter de double-compter au prochain tick.
    ///
    /// Ne touche pas a `credited_seconds` (reserve aux transitions AFK)
    /// pour eviter un double paiement lors du `voice_leave` final.
    pub async fn credit_active_sessions(&self) -> Vec<(u64, u64, u64, u64)> {
        let mut sessions = self.voice_sessions.write().await;
        let now = Utc::now().timestamp();
        let mut credited = Vec::new();
        for ((gid, uid), session) in sessions.iter_mut() {
            if let Some(since) = session.active_since {
                let delta = (now - since).max(0) as u64;
                if delta > 0 {
                    credited.push((*gid, *uid, delta, session.channel_id));
                    session.active_since = Some(now);
                }
            }
        }
        credited
    }

    /// Hydrate les sessions in-memory depuis les voice states Discord
    /// au demarrage du bot, pour ne pas perdre le temps des users
    /// deja en vocal quand le bot redemarre. Idempotent : n'ecrase pas
    /// une session existante.
    pub async fn hydrate(&self, guild_id: u64, user_id: u64, channel_id: u64, is_afk: bool) {
        let mut sessions = self.voice_sessions.write().await;
        sessions.entry((guild_id, user_id)).or_insert_with(|| {
            let active_since = if is_afk {
                None
            } else {
                Some(Utc::now().timestamp())
            };
            VoiceSession {
                active_since,
                credited_seconds: 0,
                channel_id,
            }
        });
    }

    /// Enregistre la sortie d'un utilisateur du salon vocal, cumule le temps
    /// actif (hors AFK) et retourne les secondes comptabilisees pour l'XP.
    pub async fn voice_leave(&self, guild_id: u64, user_id: u64) -> u64 {
        let mut sessions = self.voice_sessions.write().await;
        if let Some(session) = sessions.remove(&(guild_id, user_id)) {
            let tail = match session.active_since {
                Some(since) => (Utc::now().timestamp() - since).max(0) as u64,
                None => 0, // etait en AFK au moment du depart
            };
            let duration = session.credited_seconds.saturating_add(tail);
            let mut stats = self.stats.write().await;
            stats.entry((guild_id, user_id)).or_default().voice_seconds += duration;
            duration
        } else {
            0
        }
    }

    /// Récupère les stats d'un utilisateur (inclut le temps vocal en cours).
    pub async fn get_user_stats(&self, guild_id: u64, user_id: u64) -> UserStats {
        let stats = self.stats.read().await;
        let mut result = stats.get(&(guild_id, user_id)).cloned().unwrap_or_default();

        // Ajouter le temps de la session vocale en cours (hors AFK)
        let sessions = self.voice_sessions.read().await;
        if let Some(session) = sessions.get(&(guild_id, user_id)) {
            let tail = match session.active_since {
                Some(since) => (Utc::now().timestamp() - since).max(0) as u64,
                None => 0,
            };
            result.voice_seconds += session.credited_seconds + tail;
        }

        result
    }

    /// Récupère les stats de tous les utilisateurs d'un serveur.
    pub async fn get_guild_stats(&self, guild_id: u64) -> Vec<(u64, UserStats)> {
        let stats = self.stats.read().await;
        let sessions = self.voice_sessions.read().await;
        let now = Utc::now().timestamp();

        let mut result: HashMap<u64, UserStats> = HashMap::new();

        for (&(gid, uid), s) in stats.iter() {
            if gid == guild_id {
                result.insert(uid, s.clone());
            }
        }

        // Ajouter les sessions vocales en cours (hors AFK)
        for (&(gid, uid), session) in sessions.iter() {
            if gid == guild_id {
                let tail = match session.active_since {
                    Some(since) => (now - since).max(0) as u64,
                    None => 0,
                };
                result.entry(uid).or_default().voice_seconds += session.credited_seconds + tail;
            }
        }

        let mut entries: Vec<_> = result.into_iter().collect();
        entries.sort_by_key(|e| std::cmp::Reverse(e.1.message_count));
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_messages() {
        let tracker = StatsTracker::new();
        tracker.record_message(1, 100).await;
        tracker.record_message(1, 100).await;
        tracker.record_message(1, 100).await;

        let stats = tracker.get_user_stats(1, 100).await;
        assert_eq!(stats.message_count, 3);
        assert_eq!(stats.voice_seconds, 0);
    }

    #[tokio::test]
    async fn test_different_users() {
        let tracker = StatsTracker::new();
        tracker.record_message(1, 100).await;
        tracker.record_message(1, 100).await;
        tracker.record_message(1, 200).await;

        let s1 = tracker.get_user_stats(1, 100).await;
        let s2 = tracker.get_user_stats(1, 200).await;
        assert_eq!(s1.message_count, 2);
        assert_eq!(s2.message_count, 1);
    }

    #[tokio::test]
    async fn test_different_guilds() {
        let tracker = StatsTracker::new();
        tracker.record_message(1, 100).await;
        tracker.record_message(2, 100).await;

        let s1 = tracker.get_user_stats(1, 100).await;
        let s2 = tracker.get_user_stats(2, 100).await;
        assert_eq!(s1.message_count, 1);
        assert_eq!(s2.message_count, 1);
    }

    #[tokio::test]
    async fn test_voice_leave_without_join() {
        let tracker = StatsTracker::new();
        let duration = tracker.voice_leave(1, 100).await;
        assert_eq!(duration, 0);
    }

    #[tokio::test]
    async fn test_voice_join_afk_then_leave_gives_zero() {
        let tracker = StatsTracker::new();
        tracker.voice_join(1, 100, 0, true).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let duration = tracker.voice_leave(1, 100).await;
        assert_eq!(
            duration, 0,
            "une session entierement AFK ne doit pas crediter de temps"
        );
    }

    #[tokio::test]
    async fn test_voice_active_then_afk_credit_preserved() {
        let tracker = StatsTracker::new();
        tracker.voice_join(1, 100, 0, false).await;
        // bascule AFK immediatement : le tail actif (~0s) est credit
        tracker.set_voice_afk(1, 100, true).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        // sort encore AFK : les 50ms ne doivent pas compter
        let duration = tracker.voice_leave(1, 100).await;
        assert!(duration < 2, "les 50ms AFK ne doivent pas etre comptees");
    }

    #[tokio::test]
    async fn test_voice_join_then_leave() {
        let tracker = StatsTracker::new();
        tracker.voice_join(1, 100, 0, false).await;
        // Sleep tres court pour avoir une duree > 0
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let duration = tracker.voice_leave(1, 100).await;
        // La duree en secondes sera 0 (< 1s) mais le code ne doit pas panic
        assert!(duration < 2);
    }

    #[tokio::test]
    async fn test_guild_stats_sorted_by_messages() {
        let tracker = StatsTracker::new();
        tracker.record_message(1, 100).await;
        tracker.record_message(1, 200).await;
        tracker.record_message(1, 200).await;
        tracker.record_message(1, 200).await;

        let guild_stats = tracker.get_guild_stats(1).await;
        assert_eq!(guild_stats.len(), 2);
        // uid 200 a 3 messages, uid 100 a 1 → 200 en premier
        assert_eq!(guild_stats[0].0, 200);
        assert_eq!(guild_stats[0].1.message_count, 3);
        assert_eq!(guild_stats[1].0, 100);
    }

    #[tokio::test]
    async fn test_unknown_user_returns_default() {
        let tracker = StatsTracker::new();
        let stats = tracker.get_user_stats(1, 999).await;
        assert_eq!(stats.message_count, 0);
        assert_eq!(stats.voice_seconds, 0);
    }
}
