//! Membre du mois.
//!
//! Designe par le staff, jamais calcule sur l'activite. Ce qu'on veut
//! distinguer — accueillir les nouveaux, relancer un vocal mort, depanner
//! quelqu'un — ne se mesure pas en nombre de messages ; un classement
//! automatique recompenserait le bavardage.
//!
//! `reason` est obligatoire pour la meme raison : sans le pourquoi, la
//! section n'affiche qu'un nom, et la distinction ne veut plus rien dire.

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spotlight {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub avatar: Option<String>,
    /// Periode au format `YYYY-MM`.
    pub period: String,
    pub reason: String,
    pub chosen_by: String,
    pub created_at: DateTime<Utc>,
}

/// Periode d'un instant donne, au format stocke en base.
pub fn period_of(when: DateTime<Utc>) -> String {
    format!("{:04}-{:02}", when.year(), when.month())
}

/// La periode est-elle bien formee ? Le meme controle existe en base
/// (CHECK) ; il est double ici pour rendre l'erreur exploitable cote API
/// plutot que de laisser remonter une violation de contrainte.
pub fn is_valid_period(period: &str) -> bool {
    let bytes = period.as_bytes();
    bytes.len() == 7
        && bytes[4] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..].iter().all(u8::is_ascii_digit)
        && matches!(period[5..].parse::<u32>(), Ok(m) if (1..=12).contains(&m))
}

#[derive(Debug, Clone)]
pub struct UpsertSpotlightCommand {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub avatar: Option<String>,
    /// Absente = mois courant.
    pub period: Option<String>,
    pub reason: String,
    pub chosen_by: String,
}

#[cfg(test)]
#[path = "tests/spotlight.rs"]
mod tests;
