use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Statuts possibles d'un ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketStatus {
    Open,
    Pending,
    Closed,
}

impl TicketStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Pending => "pending",
            Self::Closed => "closed",
        }
    }

    /// Parse un statut depuis une string. Retourne `None` si invalide.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(Self::Open),
            "pending" => Some(Self::Pending),
            "closed" => Some(Self::Closed),
            _ => None,
        }
    }

    /// Liste des valeurs valides (pour les messages d'erreur).
    pub const VALID_VALUES: &'static [&'static str] = &["open", "pending", "closed"];

    /// Garde de transition d'etat (machine a etats des tickets).
    ///
    /// Regles :
    /// - fermer (`-> Closed`) est toujours autorise depuis n'importe quel etat ;
    /// - un ticket `Closed` ne peut etre reouvert que **explicitement** vers
    ///   `Open` (action de reouverture) — jamais vers `Pending` (ce qui
    ///   reviendrait a une reouverture silencieuse via une simple reponse) ;
    /// - entre `Open` et `Pending`, les transitions sont libres (incl. no-op).
    pub fn can_transition(from: Self, to: Self) -> bool {
        use TicketStatus::*;
        match (from, to) {
            // Fermeture toujours possible.
            (_, Closed) => true,
            // Reouverture d'un ticket ferme : uniquement vers Open, explicitement.
            (Closed, Open) => true,
            (Closed, Pending) => false,
            // Transitions libres tant que le ticket n'est pas ferme.
            (Open, _) | (Pending, _) => true,
        }
    }
}

impl fmt::Display for TicketStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
#[path = "tests/ticket_status.rs"]
mod tests;
