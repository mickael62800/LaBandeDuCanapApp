//! Mesure des faux positifs de l'automod (lecture seule).
//!
//! Le handler est mince : il parse la query, delegue l'agregation au use case
//! (`ManageAutomodReviewsUseCase::fp_stats`, ou vit toute la regle metier + le
//! SQL cote adapter Postgres), puis mappe le resultat domaine vers le DTO HTTP.

use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use crate::sentinel::bootstrap::state::ModerationState;
use platform_core::sentinel::domain::entities::moderation::review::automod::FpActionStat;
use platform_core::sentinel::domain::entities::moderation::review::automod::FpBucket;
use platform_core::sentinel::domain::entities::moderation::review::automod::FpFlagStat;
use platform_core::sentinel::domain::entities::moderation::review::automod::FpStats;

#[derive(Debug, Deserialize)]
pub struct FpStatsQuery {
    /// Fenetre en jours (defaut 30, borne 1..=365 par le use case).
    pub days: Option<i64>,
}

/// Stat globale ou par cat/action.
#[derive(Debug, Serialize)]
pub struct FpBucketDto {
    pub total: i64,
    pub overturned: i64,
    pub ignored: i64,
    pub fp_rate: f64,
}

impl From<&FpBucket> for FpBucketDto {
    fn from(b: &FpBucket) -> Self {
        Self {
            total: b.total,
            overturned: b.overturned,
            ignored: b.ignored,
            fp_rate: b.fp_rate,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FpFlagStatDto {
    pub flag: String,
    pub total: i64,
    pub overturned: i64,
    pub ignored: i64,
    pub fp_rate: f64,
}

impl From<FpFlagStat> for FpFlagStatDto {
    fn from(s: FpFlagStat) -> Self {
        Self {
            flag: s.flag,
            total: s.total,
            overturned: s.overturned,
            ignored: s.ignored,
            fp_rate: s.fp_rate,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FpActionStatDto {
    pub suggested_action: String,
    pub total: i64,
    pub overturned: i64,
    pub ignored: i64,
    pub fp_rate: f64,
}

impl From<FpActionStat> for FpActionStatDto {
    fn from(s: FpActionStat) -> Self {
        Self {
            suggested_action: s.suggested_action,
            total: s.total,
            overturned: s.overturned,
            ignored: s.ignored,
            fp_rate: s.fp_rate,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FpStatsDto {
    pub days: i64,
    /// True si l'echantillon a ete tronque (stats approximatives).
    pub capped: bool,
    pub overall: FpBucketDto,
    pub by_flag: Vec<FpFlagStatDto>,
    pub by_suggested_action: Vec<FpActionStatDto>,
}

impl From<FpStats> for FpStatsDto {
    fn from(s: FpStats) -> Self {
        Self {
            days: s.days,
            capped: s.capped,
            overall: FpBucketDto::from(&s.overall),
            by_flag: s.by_flag.into_iter().map(Into::into).collect(),
            by_suggested_action: s.by_suggested_action.into_iter().map(Into::into).collect(),
        }
    }
}

/// GET /api/automod/{guild_id}/fp-stats?days=30
///
/// Agrege les reviews terminales (applied/ignored/decided) de la fenetre et
/// mesure le taux de faux positifs (over-block) global, par flag detecteur, et
/// par action suggeree.
pub async fn fp_stats(
    State(state): State<ModerationState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<FpStatsQuery>,
) -> Result<Json<FpStatsDto>, ApiError> {
    let days = params.days.unwrap_or(30);
    let stats = state
        .automod_reviews_uc
        .fp_stats(guild_id.as_str(), days)
        .await?;
    Ok(Json(FpStatsDto::from(stats)))
}
