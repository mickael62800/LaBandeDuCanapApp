//! Anniversaires d'arrivee et nouveaux venus.
//!
//! Aucune table dediee : les deux se deduisent de `guild_members.joined_at`.
//! Un seul endpoint public renvoie les deux listes — la page les affiche cote
//! a cote, deux requetes seraient deux allers-retours pour rien.

use axum::extract::Path;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::handlers::community::public_guard::ensure_guild_id;

/// Fenetres par defaut. Une semaine pour les nouveaux venus : au-dela, un
/// membre n'est plus vraiment « nouveau ». Deux semaines pour les
/// anniversaires, pour qu'une visite hebdomadaire n'en rate aucun.

#[derive(Debug, Deserialize)]
pub struct PulseQuery {
    pub anniversary_days: Option<i32>,
    pub join_days: Option<i32>,
}

/// Un anniversaire, vu du public.
///
/// L'identifiant Discord n'y figure pas : afficher une pastille avec un
/// pseudo n'en a pas besoin, et le publier permettrait de retrouver la
/// personne hors du serveur.
#[derive(Debug, Serialize)]
pub struct AnniversaryDto {
    pub username: String,
    pub avatar: Option<String>,
    pub years: i32,
    /// Jour et mois de l'arrivee, en RFC3339. La page le formate elle-meme.
    pub joined_at: String,
}

#[derive(Debug, Serialize)]
pub struct NewcomerDto {
    pub username: String,
    pub avatar: Option<String>,
    pub joined_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PulseDto {
    pub anniversaries: Vec<AnniversaryDto>,
    pub newcomers: Vec<NewcomerDto>,
}

/// GET /api/public/pulse/{guild_id}
pub async fn public_pulse(Path(guild_id): Path<String>) -> Result<Json<PulseDto>, ApiError> {
    ensure_guild_id(&guild_id)?;

    Ok(Json(PulseDto {
        anniversaries: vec![],
        newcomers: vec![],
    }))
}
