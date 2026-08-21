//! Endpoint GET /api/games/servers/{server_id}/perf-history — historique de
//! surveillance, resume par tranches.
//!
//! Le controle de sante ecrit un point toutes les trente secondes. Une journee
//! en represente donc 2 880, et une semaine 20 000 : les servir tels quels
//! saturerait autant le reseau que le graphique, large de quelques centaines de
//! pixels. C'est la base qui resume, et cet endpoint qui decide en combien de
//! tranches.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::nexus::adapters::inbound::http::handlers::ApiError;
use crate::nexus::bootstrap::AppState;
use platform_core::nexus::domain::entities::game::server::PointDeSurveillance;

/// Plage minimale : cinq minutes. En deca, l'onglet dispose deja des chiffres
/// en direct, rafraichis toutes les cinq secondes.
const PLAGE_MIN_SECS: i64 = 300;

/// Plage maximale : trente jours. Au-dela, il n'y a de toute facon plus rien —
/// la purge garde sept jours par defaut.
const PLAGE_MAX_SECS: i64 = 30 * 24 * 3600;

/// Nombre de tranches vise quand l'appelant ne choisit pas son pas.
///
/// Soixante points remplissent un graphique de carte sans le rendre illisible :
/// une demi-heure donne alors une tranche de trente secondes, une journee une
/// tranche d'un quart d'heure.
const TRANCHES_VISEES: i64 = 60;

/// Plafond dur du nombre de tranches renvoyees.
///
/// Un appelant qui demanderait sept jours par pas de dix secondes obtiendrait
/// 60 000 points : le pas est alors elargi jusqu'a tenir dans cette borne.
/// Refuser serait plus brutal que servir une courbe un peu moins fine.
const TRANCHES_MAX: i64 = 400;

#[derive(Debug, Deserialize)]
pub struct HistoriqueQuery {
    /// Profondeur d'historique, en secondes.
    pub range_secs: Option<i64>,
    /// Largeur d'une tranche, en secondes. Absent : calculee pour la plage.
    pub step_secs: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PointDto {
    pub horodatage: String,
    pub cpu_percent: Option<f64>,
    pub memory_used_mb: Option<f64>,
    pub memory_limit_mb: Option<f64>,
    pub rcon_latency_ms: Option<i32>,
    pub net_rx_bytes_per_sec: Option<f64>,
    pub net_tx_bytes_per_sec: Option<f64>,
    pub player_count: Option<i32>,
}

impl From<PointDeSurveillance> for PointDto {
    fn from(p: PointDeSurveillance) -> Self {
        Self {
            horodatage: p.horodatage.to_rfc3339(),
            cpu_percent: p.cpu_percent,
            memory_used_mb: p.memory_used_mb,
            memory_limit_mb: p.memory_limit_mb,
            rcon_latency_ms: p.rcon_latency_ms,
            net_rx_bytes_per_sec: p.net_rx_bytes_per_sec,
            net_tx_bytes_per_sec: p.net_tx_bytes_per_sec,
            player_count: p.player_count,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HistoriqueDto {
    pub points: Vec<PointDto>,
    /// Plage et pas REELLEMENT appliques, qui ne sont pas toujours ceux
    /// demandes. L'ecran les affiche : une courbe dont le pas a ete elargi sans
    /// le dire donnerait l'impression d'avoir perdu des mesures.
    pub range_secs: i64,
    pub step_secs: i64,
}

/// Choisit le pas des tranches.
///
/// Retourne le pas effectif, borne pour que le nombre de tranches reste lisible
/// et transportable.
pub fn pas_effectif(plage_secs: i64, demande: Option<i64>) -> i64 {
    let plage = plage_secs.clamp(PLAGE_MIN_SECS, PLAGE_MAX_SECS);
    // Sans demande, on vise un nombre de tranches plutot qu'une duree fixe :
    // c'est la lisibilite du graphique qui compte, pas l'unite de temps.
    let souhaite = demande.unwrap_or_else(|| (plage / TRANCHES_VISEES).max(1));
    let souhaite = souhaite.max(1);
    // Le pas ne peut pas etre plus fin que la mesure elle-meme (30 s) sans
    // fabriquer des tranches vides entre deux releves.
    let plancher = (plage / TRANCHES_MAX).max(30);
    souhaite.max(plancher).min(plage)
}

pub async fn get_perf_history(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<HistoriqueQuery>,
) -> Result<Json<HistoriqueDto>, ApiError> {
    let plage = q
        .range_secs
        .unwrap_or(3600)
        .clamp(PLAGE_MIN_SECS, PLAGE_MAX_SECS);
    let pas = pas_effectif(plage, q.step_secs);

    let points = state
        .game_server_repo
        .history(server_id, plage, pas)
        .await?;

    Ok(Json(HistoriqueDto {
        points: points.into_iter().map(PointDto::from).collect(),
        range_secs: plage,
        step_secs: pas,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sans_demande_le_pas_vise_une_soixantaine_de_tranches() {
        // Une heure -> une minute par tranche ; une journee -> un quart d'heure.
        assert_eq!(pas_effectif(3600, None), 60);
        assert_eq!(pas_effectif(24 * 3600, None), 1440);
    }

    #[test]
    fn un_pas_trop_fin_est_elargi_plutot_que_refuse() {
        // Sept jours par pas de dix secondes, ce serait 60 000 points. On sert
        // une courbe moins fine plutot que rien : l'appelant lit le pas
        // reellement applique dans la reponse.
        let sept_jours = 7 * 24 * 3600;
        let pas = pas_effectif(sept_jours, Some(10));
        assert_eq!(pas, sept_jours / TRANCHES_MAX);
        assert!(sept_jours / pas <= TRANCHES_MAX);
    }

    #[test]
    fn le_pas_ne_descend_jamais_sous_la_frequence_de_mesure() {
        // Le controle de sante ecrit toutes les 30 s : des tranches de 5 s
        // fabriqueraient des trous entre deux releves.
        assert_eq!(pas_effectif(PLAGE_MIN_SECS, Some(5)), 30);
    }

    #[test]
    fn le_pas_ne_depasse_jamais_la_plage() {
        // Sinon la reponse tiendrait en un point unique, sans forme.
        assert_eq!(pas_effectif(600, Some(100_000)), 600);
    }
}
