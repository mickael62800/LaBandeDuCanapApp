//! Recherche de joueurs — « je cherche 2 personnes pour Valheim ce soir ».
//!
//! Une annonce est ephemere : passe l'horaire annonce, elle n'interesse plus
//! personne. Elle porte donc sa propre date d'expiration plutot que d'attendre
//! qu'un humain la supprime, ce que personne ne fait jamais.
//!
//! Le « quand » est du texte libre (`when_text`) et non un horodatage. La
//! majorite des annonces reelles disent « ce soir », « le week-end » ou
//! « quand vous voulez » : forcer un timestamp aurait oblige a inventer une
//! heure, donc a mentir.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Duree de vie par defaut d'une annonce sans expiration explicite.
///
/// Deux jours : assez pour couvrir « ce soir » et « demain », assez court
/// pour que la section ne se transforme pas en cimetiere d'annonces mortes.
pub const DEFAULT_LIFETIME_HOURS: i64 = 48;

/// Plafond, pour qu'une annonce ne puisse pas squatter la page un mois.
pub const MAX_LIFETIME_HOURS: i64 = 24 * 14;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfgPost {
    pub id: Uuid,
    pub guild_id: String,
    pub author_id: String,
    pub author_name: String,
    pub game: String,
    /// Rattachement facultatif a un serveur de jeu Nexus, pour afficher sa
    /// jaquette et son etat.
    pub game_server_id: Option<Uuid>,
    /// Nombre de personnes RECHERCHEES, pas la taille du groupe.
    pub slots: i32,
    pub when_text: String,
    pub description: Option<String>,
    pub is_open: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    /// Membres qui se sont manifestes. Charge avec l'annonce : on ne montre
    /// jamais une annonce sans dire qui vient.
    pub interested: Vec<LfgInterest>,
}

impl LfgPost {
    /// Une annonce expiree n'a plus a s'afficher, meme si personne ne l'a
    /// fermee.
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at <= now
    }

    /// Visible sur la page : ouverte ET non expiree.
    pub fn is_live(&self, now: DateTime<Utc>) -> bool {
        self.is_open && !self.is_expired(now)
    }

    /// Places encore a pourvoir. Ne descend pas sous zero : afficher
    /// « -1 place » n'aurait aucun sens si trois personnes repondent a une
    /// annonce qui en cherchait deux.
    pub fn remaining_slots(&self) -> i32 {
        (self.slots - self.interested.len() as i32).max(0)
    }

    /// Le groupe est-il complet ? Sert a marquer l'annonce plutot qu'a la
    /// masquer : quelqu'un peut toujours vouloir se joindre en plus.
    pub fn is_full(&self) -> bool {
        self.remaining_slots() == 0
    }

    pub fn has_interest_from(&self, user_id: &str) -> bool {
        self.interested.iter().any(|i| i.user_id == user_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfgInterest {
    pub user_id: String,
    pub username: String,
    pub joined_at: DateTime<Utc>,
}

/// Commande de creation/mise a jour. Champs bruts, valides par le service.
#[derive(Debug, Clone)]
pub struct UpsertLfgCommand {
    pub guild_id: String,
    pub author_id: String,
    pub author_name: String,
    pub game: String,
    pub game_server_id: Option<Uuid>,
    pub slots: i32,
    pub when_text: String,
    pub description: Option<String>,
    /// Absent = duree de vie par defaut.
    pub expires_at: Option<DateTime<Utc>>,
}

impl UpsertLfgCommand {
    /// Expiration effective, bornee. Centralise ici plutot que dans le
    /// service pour que la valeur par defaut et le plafond restent avec la
    /// definition du concept.
    pub fn resolved_expiry(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        let max = now + Duration::hours(MAX_LIFETIME_HOURS);
        match self.expires_at {
            // Une expiration deja passee serait une annonce morte-nee : on la
            // ramene a la duree par defaut plutot que de refuser la creation.
            Some(t) if t > now => t.min(max),
            _ => now + Duration::hours(DEFAULT_LIFETIME_HOURS),
        }
    }
}

#[cfg(test)]
#[path = "tests/lfg.rs"]
mod tests;
