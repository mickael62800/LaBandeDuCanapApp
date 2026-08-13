use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrikeThreshold {
    pub strikes: u32,
    pub action: String,
    pub duration: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrikeConfig {
    pub guild_id: GuildId,
    pub window_secs: i64,
    pub thresholds: Vec<StrikeThreshold>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl StrikeConfig {
    pub fn default_for_guild(guild_id: &str) -> Self {
        let now = Utc::now();
        Self {
            guild_id: guild_id.to_string().into(),
            window_secs: 3600,
            thresholds: vec![],
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStrike {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub reason: String,
    pub source: String,
    pub infraction_id: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Logique de correspondance d'echelle (ladder) partagee : trie les seuils par
/// nombre de strikes decroissant et retourne l'action + la duree du PREMIER
/// seuil atteint (`active_count >= seuil.strikes`). Fonction pure, sans I/O.
///
/// Centralise la regle jadis inline dans `ManageStrikesService::add_strike`
/// afin que le copilote de moderation la reutilise a l'identique (pas de
/// duplication de la logique d'escalade).
pub fn escalation_for(
    thresholds: &[StrikeThreshold],
    active_count: u32,
) -> Option<(String, Option<u64>)> {
    let mut sorted = thresholds.to_vec();
    sorted.sort_by_key(|t| std::cmp::Reverse(t.strikes));
    sorted
        .iter()
        .find(|t| active_count >= t.strikes)
        .map(|t| (t.action.clone(), t.duration))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrikeResult {
    pub strike: UserStrike,
    pub active_count: u32,
    pub escalation_action: Option<String>,
    pub escalation_duration: Option<u64>,
}

impl StrikeResult {
    /// Indique si cette resolution de strike doit declencher un broadcast
    /// `strike_added` (seuil d'escalade franchi). Centralise la regle metier
    /// "broadcast ssi une action d'escalade a ete decidee" pour que les
    /// handlers n'aient pas a la redefinir via `escalation_action.is_some()`.
    pub fn should_trigger_escalation_broadcast(&self) -> bool {
        self.escalation_action.is_some()
    }
}

#[cfg(test)]
#[path = "tests/strikes.rs"]
mod tests;
