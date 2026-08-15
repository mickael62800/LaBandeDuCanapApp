//! Anniversaires d'arrivee et nouveaux venus.
//!
//! Aucune table dediee : les deux se deduisent de `guild_members.joined_at`.
//! Un seul endpoint public renvoie les deux listes — la page les affiche cote
//! a cote, deux requetes seraient deux allers-retours pour rien.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::handlers::community::public_guard::ensure_guild_id;
use crate::sentinel::bootstrap::state::CommunityState;

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
pub async fn public_pulse(
    State(state): State<CommunityState>,
    Path(guild_id): Path<String>,
    Query(query): Query<PulseQuery>,
) -> Result<Json<PulseDto>, ApiError> {
    ensure_guild_id(&guild_id)?;

    let anniv_days = query.anniversary_days.unwrap_or(14);
    let join_days = query.join_days.unwrap_or(30);

    let anniversaries_data = state
        .members_uc
        .upcoming_anniversaries(&guild_id, anniv_days)
        .await?;

    let newcomers_data = state
        .members_uc
        .recent_joins(&guild_id, join_days, 20)
        .await?;

    let anniversaries = anniversaries_data
        .into_iter()
        .map(|a| AnniversaryDto {
            username: a.username,
            avatar: a.avatar,
            years: a.years,
            joined_at: a.joined_at.to_rfc3339(),
        })
        .collect();

    let newcomers = newcomers_data
        .into_iter()
        .map(|m| NewcomerDto {
            username: m.display_name.unwrap_or(m.username),
            avatar: m.avatar,
            joined_at: m.joined_at.map(|d| d.to_rfc3339()),
        })
        .collect();

    Ok(Json(PulseDto {
        anniversaries,
        newcomers,
    }))
}
