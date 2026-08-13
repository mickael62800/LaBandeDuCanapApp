//! Detection d'anomalie de moderation (mass ban / delete / role change).
//!
//! Regle metier remontee depuis le bot : sur une fenetre glissante, si le
//! nombre d'evenements d'une categorie depasse un seuil, on declenche une
//! alerte. Le CALCUL (comptage fenetre) vit dans un adapter serveur ; la
//! DECISION (seuil + type d'alerte + reset) vit ici, dans le coeur.

/// Seuils de detection d'anomalie, resolus per-guild cote appelant.
#[derive(Debug, Clone, Copy)]
pub struct AnomalyThresholds {
    pub mass_ban: usize,
    pub mass_delete: usize,
    pub mass_role_change: usize,
}

impl Default for AnomalyThresholds {
    fn default() -> Self {
        Self {
            mass_ban: 5,
            mass_delete: 20,
            mass_role_change: 10,
        }
    }
}

impl AnomalyThresholds {
    /// Seuil applicable a une categorie d'evenement. Les categories inconnues
    /// ne declenchent jamais d'alerte (`usize::MAX`).
    pub fn threshold_for(&self, category: &str) -> usize {
        match category {
            "ban" | "kick" => self.mass_ban,
            "delete" => self.mass_delete,
            "role_change" => self.mass_role_change,
            _ => usize::MAX,
        }
    }
}

/// Alerte d'anomalie decidee par le coeur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModerationAnomaly {
    /// Type d'anomalie normalise, ex: `mass_ban`, `mass_delete`.
    pub anomaly_type: String,
    /// Nombre d'evenements dans la fenetre au moment du declenchement.
    pub count: usize,
    /// Taille de la fenetre glissante en secondes.
    pub window_secs: u64,
}
