//! Analyse de securite : raid patterns, comptes suspects, alt detection.
//!
//! Fonctions pures (aucune IO) — migrees depuis security-bot pour
//! centraliser la logique metier cote API.

/// Info d'un join recent (envoyee par le bot).
#[derive(Debug, Clone)]
pub struct JoinInfo {
    pub username: String,
    pub has_avatar: bool,
    pub account_created_timestamp: i64,
}

/// Resultat de l'analyse raid.
#[derive(Debug, Clone)]
pub struct RaidAnalysis {
    pub similar_names: bool,
    pub high_default_avatar_ratio: bool,
    pub clustered_creation: bool,
    pub score: u32,
}

// ── Levenshtein ──

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let (a_len, b_len) = (a_chars.len(), b_chars.len());
    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0usize; b_len + 1];
    for i in 1..=a_len {
        curr[0] = i;
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b_len]
}

// ── Noms similaires ──

pub fn has_similar_usernames(names: &[String], max_distance: usize) -> bool {
    if names.len() < 2 {
        return false;
    }
    let capped = if names.len() > 50 {
        &names[..50]
    } else {
        names
    };
    let lowered: Vec<String> = capped.iter().map(|n| n.to_lowercase()).collect();
    for i in 0..lowered.len() {
        for j in (i + 1)..lowered.len() {
            if levenshtein(&lowered[i], &lowered[j]) <= max_distance {
                return true;
            }
        }
    }
    false
}

// ── Cluster de creation ──

pub fn are_creations_clustered(timestamps: &[i64], max_spread_secs: i64) -> bool {
    if timestamps.len() < 2 {
        return false;
    }
    let min = timestamps.iter().min().copied().unwrap_or(0);
    let max = timestamps.iter().max().copied().unwrap_or(0);
    (max - min) <= max_spread_secs
}

// ── Analyse raid ──

pub fn analyze_joins(
    joins: &[JoinInfo],
    name_distance: usize,
    creation_spread_secs: i64,
) -> RaidAnalysis {
    if joins.len() < 2 {
        return RaidAnalysis {
            similar_names: false,
            high_default_avatar_ratio: false,
            clustered_creation: false,
            score: 0,
        };
    }

    let names: Vec<String> = joins.iter().map(|j| j.username.clone()).collect();
    let similar_names = has_similar_usernames(&names, name_distance);

    let default_count = joins.iter().filter(|j| !j.has_avatar).count();
    let high_default_avatar_ratio = (default_count as f64 / joins.len() as f64) > 0.5;

    let timestamps: Vec<i64> = joins.iter().map(|j| j.account_created_timestamp).collect();
    let clustered_creation = are_creations_clustered(&timestamps, creation_spread_secs);

    let mut score: u32 = 0;
    if similar_names {
        score += 40;
    }
    if high_default_avatar_ratio {
        score += 30;
    }
    if clustered_creation {
        score += 30;
    }

    RaidAnalysis {
        similar_names,
        high_default_avatar_ratio,
        clustered_creation,
        score,
    }
}

// ── Politique auto-vs-suggest (mode hybride anti-raid) ──

/// Mode de reponse anti-raid configure par le serveur.
///
/// - `Auto` : la reponse guild-wide (lockdown/slowmode/verification) est
///   appliquee directement.
/// - `Suggest` : la reponse est seulement proposee au staff (boutons
///   confirmer/ignorer).
/// - `Hybrid` : auto si le raid est massif (flood de vitesse OU score eleve),
///   sinon suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidMode {
    Auto,
    Suggest,
    Hybrid,
}

impl RaidMode {
    /// Parse depuis la valeur de config (`auto` | `suggest` | `hybrid`).
    /// Toute valeur inconnue (ou vide) retombe sur `Hybrid` (defaut owner).
    pub fn from_config(value: &str) -> Self {
        match value {
            "auto" => RaidMode::Auto,
            "suggest" => RaidMode::Suggest,
            _ => RaidMode::Hybrid,
        }
    }
}

/// Resultat de la politique : appliquer directement ou suggerer au staff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidResponseMode {
    Auto,
    Suggest,
}

/// Politique PURE auto-vs-suggest pour la reponse GUILD-WIDE anti-raid.
///
/// - `Auto` => toujours `Auto`.
/// - `Suggest` => toujours `Suggest`.
/// - `Hybrid` => `Auto` si `is_velocity_raid` OU `raid_score >= auto_threshold`,
///   sinon `Suggest`.
pub fn raid_response_mode(
    raid_score: i32,
    is_velocity_raid: bool,
    mode: RaidMode,
    auto_threshold: i32,
) -> RaidResponseMode {
    match mode {
        RaidMode::Auto => RaidResponseMode::Auto,
        RaidMode::Suggest => RaidResponseMode::Suggest,
        RaidMode::Hybrid => {
            if is_velocity_raid || raid_score >= auto_threshold {
                RaidResponseMode::Auto
            } else {
                RaidResponseMode::Suggest
            }
        }
    }
}

// ── Check age compte ──

pub fn is_account_suspicious(account_created_timestamp: i64, min_age_secs: u64) -> bool {
    let now = chrono::Utc::now().timestamp();
    let age = now - account_created_timestamp;
    if age < 0 {
        return true;
    }
    (age as u64) < min_age_secs
}

// ── Alt detection (contre les bans recents en DB) ──

#[derive(Debug, Clone)]
pub struct BannedUserInfo {
    pub username: String,
    pub account_created_timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct AltAnalysis {
    pub similar_to_banned: Option<String>,
    pub creation_near_banned: Option<String>,
}

impl AltAnalysis {
    pub fn is_suspicious(&self) -> bool {
        self.similar_to_banned.is_some() || self.creation_near_banned.is_some()
    }
}

pub fn check_alt_account(
    username: &str,
    account_created_timestamp: i64,
    recent_bans: &[BannedUserInfo],
    name_distance: usize,
    creation_cluster_secs: i64,
) -> AltAnalysis {
    let mut similar_to_banned = None;
    let mut creation_near_banned = None;
    let username_lower = username.to_lowercase();

    for ban in recent_bans {
        let ban_lower = ban.username.to_lowercase();
        if levenshtein(&username_lower, &ban_lower) <= name_distance {
            similar_to_banned = Some(ban.username.clone());
        }
        let diff = (account_created_timestamp - ban.account_created_timestamp).abs();
        if diff <= creation_cluster_secs {
            creation_near_banned = Some(ban.username.clone());
        }
        if similar_to_banned.is_some() && creation_near_banned.is_some() {
            break;
        }
    }

    AltAnalysis {
        similar_to_banned,
        creation_near_banned,
    }
}

#[cfg(test)]
#[path = "tests/security_analyzer.rs"]
mod tests;
