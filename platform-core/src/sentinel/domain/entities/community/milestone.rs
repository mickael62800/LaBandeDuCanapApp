//! Anniversaires d'arrivee et nouveaux venus.
//!
//! Aucune table dediee : tout se deduit de `guild_members.joined_at`.
//! Recopier cette date ailleurs, c'est garantir qu'elle divergera le jour ou
//! quelqu'un quitte puis revient.

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

/// Un membre qui fete son arrivee dans les jours qui viennent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinAnniversary {
    pub user_id: String,
    pub username: String,
    pub avatar: Option<String>,
    pub joined_at: DateTime<Utc>,
    /// Nombre d'annees fetees. Toujours >= 1 : on ne fete pas un « 0 an ».
    pub years: i32,
}

/// Annees revolues entre l'arrivee et la date de l'anniversaire.
///
/// Le calcul se fait sur l'annee de l'ANNIVERSAIRE et non sur aujourd'hui :
/// la section affiche une fenetre glissante qui deborde sur l'annee suivante
/// (un 2 janvier consulte le 28 decembre), et compter depuis aujourd'hui
/// annoncerait alors une annee de moins.
pub fn years_at(joined_at: DateTime<Utc>, anniversary: DateTime<Utc>) -> i32 {
    (anniversary.year() - joined_at.year()).max(0)
}

/// Le 29 fevrier existe une annee sur quatre. Sans ce repli, les membres
/// arrives ce jour-la n'auraient jamais d'anniversaire affiche.
pub fn celebrated_day(month: u32, day: u32, year: i32) -> (u32, u32) {
    let bissextile = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    if month == 2 && day == 29 && !bissextile {
        (2, 28)
    } else {
        (month, day)
    }
}

#[cfg(test)]
#[path = "tests/milestone.rs"]
mod tests;
