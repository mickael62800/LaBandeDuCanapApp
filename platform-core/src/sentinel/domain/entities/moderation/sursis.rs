//! « Ban en sursis » — un membre mis en attente de bannissement (role Sursis),
//! qui dispose d'un delai pour contester avant le ban definitif.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Statut d'un sursis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SursisStatus {
    /// En cours : le membre porte le role Sursis et peut contester.
    EnSursis,
    /// Gracie : appel accepte, roles restaures.
    Gracie,
    /// Banni : delai ecoule ou ban confirme.
    Banni,
}

impl SursisStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SursisStatus::EnSursis => "en_sursis",
            SursisStatus::Gracie => "gracie",
            SursisStatus::Banni => "banni",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "en_sursis" => Some(Self::EnSursis),
            "gracie" => Some(Self::Gracie),
            "banni" => Some(Self::Banni),
            _ => None,
        }
    }
}

/// Un membre en sursis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sursis {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub reason: String,
    /// Roles a restaurer si l'appel est accepte.
    pub saved_roles: Vec<String>,
    pub channel_id: Option<String>,
    pub status: SursisStatus,
    pub expires_at: DateTime<Utc>,
}
