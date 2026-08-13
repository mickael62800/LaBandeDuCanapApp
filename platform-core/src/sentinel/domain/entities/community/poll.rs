//! Sondage communautaire.
//!
//! Un vote par personne et par sondage, garanti par la cle primaire de la
//! table des votes et non par une verification applicative : le double vote
//! est structurellement impossible, pas seulement interdit.
//!
//! Les pourcentages se calculent ici et non dans le front : deux clients
//! (site et bot) affichent les memes resultats, ils doivent arrondir pareil.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Palette de repli, dans l'ordre des options. Une option sans couleur
/// choisie reste lisible au lieu de tomber sur un gris indifferencie.
pub const DEFAULT_COLORS: [&str; 6] = ["a855f7", "22c55e", "f43f5e", "f39c12", "38bdf8", "14b8a6"];

pub const MIN_OPTIONS: usize = 2;
pub const MAX_OPTIONS: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poll {
    pub id: Uuid,
    pub guild_id: String,
    pub question: String,
    pub description: Option<String>,
    pub closes_at: DateTime<Utc>,
    pub is_closed: bool,
    pub is_public: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub options: Vec<PollOption>,
}

impl Poll {
    /// Un sondage se ferme soit a la main, soit par sa date. Les deux
    /// comptent : sans la date, un sondage oublie resterait ouvert des mois.
    pub fn is_open(&self, now: DateTime<Utc>) -> bool {
        !self.is_closed && self.closes_at > now
    }

    pub fn total_votes(&self) -> i64 {
        self.options.iter().map(|o| o.votes).sum()
    }

    /// Part de chaque option, en pourcentage entier.
    ///
    /// Sans voix, on renvoie 0 partout plutot que de diviser par zero — un
    /// sondage tout juste ouvert est le cas normal, pas une erreur.
    pub fn shares(&self) -> Vec<i32> {
        let total = self.total_votes();
        if total == 0 {
            return vec![0; self.options.len()];
        }
        self.options
            .iter()
            .map(|o| ((o.votes as f64 / total as f64) * 100.0).round() as i32)
            .collect()
    }

    /// Couleur effective d'une option : la sienne, sinon la palette de repli
    /// selon sa position.
    pub fn color_at(&self, index: usize) -> String {
        self.options
            .get(index)
            .and_then(|o| o.color.clone())
            .unwrap_or_else(|| DEFAULT_COLORS[index % DEFAULT_COLORS.len()].to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollOption {
    pub id: Uuid,
    pub label: String,
    /// Hex sans `#`.
    pub color: Option<String>,
    pub position: i32,
    /// Nombre de voix, agrege par le repository.
    pub votes: i64,
}

/// Commande de creation/mise a jour. Les options sont donnees en entier :
/// modifier un sondage, c'est redefinir ses choix.
#[derive(Debug, Clone)]
pub struct UpsertPollCommand {
    pub guild_id: String,
    pub question: String,
    pub description: Option<String>,
    pub closes_at: DateTime<Utc>,
    pub is_public: bool,
    pub created_by: String,
    /// Libelle + couleur facultative, dans l'ordre d'affichage voulu.
    pub options: Vec<(String, Option<String>)>,
}

#[cfg(test)]
#[path = "tests/poll.rs"]
mod tests;
