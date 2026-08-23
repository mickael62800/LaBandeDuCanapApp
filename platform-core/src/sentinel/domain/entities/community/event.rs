//! Evenement du planning communautaire.
//!
//! Un evenement est une PLAGE, pas un instant : une saison Minecraft ou une
//! campagne Palworld tient plusieurs semaines, une soiree ponctuelle est une
//! plage de quelques heures. Toute la logique d'affichage (vue semaine ou
//! mois) se ramene donc a un chevauchement d'intervalles.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Etat de publication. Un evenement se prepare souvent avant d'etre annonce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventStatus {
    Draft,
    Published,
    Cancelled,
}

impl EventStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Cancelled => "cancelled",
        }
    }

    /// Inconnu => Draft : au pire l'evenement reste invisible, jamais publie
    /// par accident.
    pub fn parse(s: &str) -> Self {
        match s {
            "published" => Self::Published,
            "cancelled" => Self::Cancelled,
            _ => Self::Draft,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityEvent {
    pub id: Uuid,
    pub guild_id: String,
    pub title: String,
    pub description: Option<String>,
    /// Jeu concerne, en texte libre : le planning doit pouvoir annoncer un jeu
    /// qui n'a pas de serveur chez nous.
    pub game: Option<String>,
    /// Couleur d'affichage (hex sans `#`).
    pub color: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub all_day: bool,
    pub is_public: bool,
    pub status: EventStatus,
    /// Serveur de jeu Nexus a l'origine de cet evenement, s'il y en a un.
    ///
    /// Sans contrainte d'integrite : `game_servers` vit dans la base `nexus`,
    /// cette table dans `discord_sentinel`. Le lien est declaratif — il permet
    /// de retrouver l'evenement d'un serveur pour le supprimer avec lui, ce qui
    /// n'etait pas possible avant : une session dont le serveur avait disparu
    /// restait annoncee des semaines sur le site public.
    pub source_server_id: Option<Uuid>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CommunityEvent {
    /// L'evenement chevauche-t-il la fenetre affichee ?
    ///
    /// Bornes volontairement inclusives d'un cote seulement (`start < to` et
    /// `end >= from`) : un evenement qui se termine pile au debut de la
    /// fenetre n'a pas a y apparaitre, alors qu'un evenement qui commence
    /// pile a la fin doit etre visible dans la fenetre suivante uniquement.
    pub fn overlaps(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> bool {
        self.starts_at < to && self.ends_at >= from
    }

    /// Nombre de jours couverts, au minimum 1. Sert a dimensionner la barre
    /// dans la vue calendrier.
    pub fn span_days(&self) -> i64 {
        (self.ends_at.date_naive() - self.starts_at.date_naive())
            .num_days()
            .max(0)
            + 1
    }

    /// Un evenement long merite un rendu different d'une soiree : au-dela de
    /// la journee, on parle de campagne.
    pub fn is_multi_day(&self) -> bool {
        self.span_days() > 1
    }
}

/// Commande de creation/mise a jour. Champs bruts, valides par le service.
#[derive(Debug, Clone)]
pub struct UpsertEventCommand {
    pub guild_id: String,
    pub title: String,
    pub description: Option<String>,
    pub game: Option<String>,
    pub color: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub all_day: bool,
    pub is_public: bool,
    pub status: EventStatus,
    pub created_by: String,
    /// Serveur de jeu Nexus a l'origine de cet evenement, s'il y en a un.
    ///
    /// Sans contrainte d'integrite : `game_servers` vit dans la base `nexus`,
    /// cette table dans `discord_sentinel`. Le lien est declaratif — il permet
    /// de retrouver l'evenement d'un serveur pour le supprimer avec lui, ce qui
    /// n'etait pas possible avant : une session dont le serveur avait disparu
    /// restait annoncee des semaines sur le site public.
    pub source_server_id: Option<Uuid>,
}

/// Reponse d'un participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventAnswer {
    Going,
    Maybe,
}

impl EventAnswer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Going => "going",
            Self::Maybe => "maybe",
        }
    }

    pub fn parse(s: &str) -> Self {
        if s == "maybe" {
            Self::Maybe
        } else {
            Self::Going
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventParticipant {
    pub event_id: Uuid,
    pub user_id: String,
    pub username: String,
    pub answer: EventAnswer,
    pub registered_at: DateTime<Utc>,
}

#[cfg(test)]
#[path = "tests/event.rs"]
mod tests;
