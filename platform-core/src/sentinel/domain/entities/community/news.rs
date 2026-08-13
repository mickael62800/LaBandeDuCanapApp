//! Annonce du site.
//!
//! A ne pas confondre avec `announcement` : celle-la pilote des messages
//! Discord recurrents postes par le bot (rappels de bump, regles). Les
//! melanger ferait remonter « pensez a bump ! » dans les nouvelles du site.
//!
//! Une nouvelle est datee (`published_at`) et peut etre epinglee. L'epingle
//! l'emporte sur la date : une information importante reste en tete meme
//! quand des nouvelles plus recentes arrivent.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Longueur au-dela de laquelle le corps est tronque dans la liste. Le texte
/// complet reste disponible sur la fiche.
pub const EXCERPT_CHARS: usize = 180;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsPost {
    pub id: Uuid,
    pub guild_id: String,
    pub title: String,
    pub body: String,
    /// Chemin RELATIF (`/imgs/...`), comme les jaquettes de jeu : une URL
    /// absolue figerait le domaine en base.
    pub image_url: Option<String>,
    pub is_pinned: bool,
    pub is_public: bool,
    pub published_at: DateTime<Utc>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

impl NewsPost {
    /// Deja publiee ? Une date future permet de preparer une nouvelle a
    /// l'avance.
    pub fn is_published(&self, now: DateTime<Utc>) -> bool {
        self.published_at <= now
    }

    /// Extrait pour la liste, coupe sur une frontiere de mot.
    ///
    /// Coupe en `chars` et non en octets : un `&body[..180]` planterait au
    /// milieu d'un caractere accentue, ce qui est la norme en francais.
    pub fn excerpt(&self) -> String {
        let body = self.body.trim();
        if body.chars().count() <= EXCERPT_CHARS {
            return body.to_string();
        }
        let coupe: String = body.chars().take(EXCERPT_CHARS).collect();
        // On recule jusqu'au dernier espace pour ne pas trancher un mot ; si
        // le texte n'en contient aucun, on garde la coupe brute.
        let fin = coupe.rfind(' ').unwrap_or(coupe.len());
        format!("{}…", coupe[..fin].trim_end())
    }
}

#[derive(Debug, Clone)]
pub struct UpsertNewsCommand {
    pub guild_id: String,
    pub title: String,
    pub body: String,
    pub image_url: Option<String>,
    pub is_pinned: bool,
    pub is_public: bool,
    /// Absente = maintenant.
    pub published_at: Option<DateTime<Utc>>,
    pub created_by: String,
}

#[cfg(test)]
#[path = "tests/news.rs"]
mod tests;
