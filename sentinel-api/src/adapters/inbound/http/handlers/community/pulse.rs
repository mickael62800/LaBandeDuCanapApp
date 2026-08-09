//! Anniversaires d'arrivee et nouveaux venus.
//!
//! Aucune table dediee : les deux se deduisent de `guild_members.joined_at`.
//! Un seul endpoint public renvoie les deux listes — la page les affiche cote
//! a cote, deux requetes seraient deux allers-retours pour rien.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::handlers::community::public_guard::ensure_guild_id;
use crate::bootstrap::state::CommunityState;

/// Fenetres par defaut. Une semaine pour les nouveaux venus : au-dela, un
/// membre n'est plus vraiment « nouveau ». Deux semaines pour les
/// anniversaires, pour qu'une visite hebdomadaire n'en rate aucun.
const DEFAULT_ANNIVERSARY_DAYS: i32 = 14;
const DEFAULT_JOIN_DAYS: i32 = 7;
const MAX_NEWCOMERS: i64 = 12;

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
pub async fn public_pulse(
    State(state): State<CommunityState>,
    Path(guild_id): Path<String>,
    Query(q): Query<PulseQuery>,
) -> Result<Json<PulseDto>, ApiError> {
    ensure_guild_id(&guild_id)?;

    let anniversaries = vec![];

    let newcomers = vec![];

    Ok(Json(PulseDto {
        anniversaries: anniversaries
            .into_iter()
            .map(|a| AnniversaryDto {
                username: a.username,
                avatar: a.avatar,
                years: a.years,
                joined_at: a.joined_at.to_rfc3339(),
            })
            .collect(),
        newcomers: newcomers
            .into_iter()
            .map(|m| NewcomerDto {
                username: m.display_name.unwrap_or(m.username),
                avatar: m.avatar,
                joined_at: m.joined_at.map(|d| d.to_rfc3339()),
            })
            .collect(),
    }))
}

